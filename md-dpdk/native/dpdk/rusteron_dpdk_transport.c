/*
 * ergon-md-dpdk DPDK kernel-bypass transport: the stable ABI (create, install,
 * close, last error) and the Aeron UDP channel transport binding table, whose
 * callbacks open and close channels and send (ARP, bandwidth allowances,
 * stream priority). Receiving is in rusteron_dpdk_poller.c; EAL and port
 * bring-up in rusteron_dpdk_runtime.c and rusteron_dpdk_eal.c.
 */
#include "rusteron_dpdk_internal.h"

#include <arpa/inet.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "util/aeron_error.h"
#include "media/aeron_udp_channel_transport.h"
/* Full driver-context layout: the counters manager is reached via
 * context->counters_manager at transport init (plan §9). bindings.h only
 * forward-declares the type. */
#include "aeron_driver_context.h"

static int rusteron_dpdk_port_bound(const rusteron_dpdk_port_table_t *table, int index, uint16_t port);
static void rusteron_dpdk_port_set_bound(rusteron_dpdk_port_table_t *table, int index, uint16_t port, int bound);
static void rusteron_dpdk_port_release(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_table_t *table, int index, uint16_t port);
static int rusteron_dpdk_ip_index(const rusteron_dpdk_port_t *port_info, uint32_t ip);
static uint16_t rusteron_dpdk_ephemeral_port(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_table_t *table, const aeron_driver_context_t *context);
static void rusteron_dpdk_transport_init_fields(
    aeron_udp_channel_transport_t *transport, aeron_driver_context_t *context,
    const aeron_udp_channel_transport_params_t *params);
/* Zeroed and cache-line aligned, so its hot fields share one line. */
static rusteron_dpdk_client_t *rusteron_dpdk_client_alloc(void)
{
    rusteron_dpdk_client_t *client = aligned_alloc(64, sizeof(rusteron_dpdk_client_t));
    if (NULL != client)
    {
        memset(client, 0, sizeof(*client));
    }
    return client;
}

static int rusteron_dpdk_transport_init_delegated(
    aeron_udp_channel_transport_t *transport,
    rusteron_dpdk_transport_t *native,
    rusteron_dpdk_port_t *port,
    const struct sockaddr_in *bind_in,
    const struct sockaddr_in *connect_in,
    struct sockaddr_storage *connect_addr,
    aeron_udp_channel_transport_params_t *params,
    aeron_driver_context_t *context);

#define RUSTERON_DPDK_BINDINGS_NAME "rusteron-dpdk-ena"
#define RUSTERON_DPDK_BINDINGS_TYPE "media"

/* Aeron never passes more than AERON_NETWORK_PUBLICATION_MAX_MESSAGES_PER_SEND
 * iovecs to one send; the batch buffer must hold a full burst regardless. */
#define RUSTERON_DPDK_MAX_MESSAGES_PER_SEND 16

/* The transport layout lives in rusteron_dpdk_internal.h. */

/* Process-lifetime native runtime. EAL can be initialized exactly once, so
 * there is at most one live transport; the binding callbacks find it here. */
static rusteron_dpdk_transport_t *rusteron_dpdk_the_runtime = NULL;

rusteron_dpdk_transport_t *rusteron_dpdk_active_runtime(void)
{
    return rusteron_dpdk_the_runtime;
}

static _Thread_local char rusteron_dpdk_error_buffer[1024];
static _Thread_local int rusteron_dpdk_error_code = RUSTERON_DPDK_ERR_OK;

void rusteron_dpdk_set_error_code(const char *message, int code)
{
    strncpy(rusteron_dpdk_error_buffer, message, sizeof(rusteron_dpdk_error_buffer) - 1);
    rusteron_dpdk_error_buffer[sizeof(rusteron_dpdk_error_buffer) - 1] = '\0';
    rusteron_dpdk_error_code = code;
}

void rusteron_dpdk_set_error(const char *message)
{
    rusteron_dpdk_set_error_code(message, RUSTERON_DPDK_ERR_NATIVE);
}

/*
 * Minimal string field validation shared by config fields that hold fixed
 * buffers. Strings must be non-empty and NUL-terminated within the buffer.
 */
static int rusteron_dpdk_config_validate_string(const char *value, size_t capacity, const char *name)
{
    if (NULL == value || '\0' == value[0])
    {
        char message[256];
        snprintf(message, sizeof(message), "config field '%s' must be a non-empty string", name);
        rusteron_dpdk_set_error(message);
        return -1;
    }

    if (memchr(value, '\0', capacity) == NULL)
    {
        char message[256];
        snprintf(message, sizeof(message), "config field '%s' is not NUL-terminated within %zu bytes", name, capacity);
        rusteron_dpdk_set_error(message);
        return -1;
    }

    return 0;
}

/* The config layout, field by field: src/ffi.rs and tests/common/mod.rs mirror
 * it and assert the same offsets, so a field moved, resized or swapped with one
 * of its size fails every build instead of passing the struct_size check. */
#define RUSTERON_DPDK_CONFIG_AT(field, offset) \
    _Static_assert(offsetof(rusteron_dpdk_config_t, field) == (offset), #field " moved: update the Rust mirrors")
RUSTERON_DPDK_CONFIG_AT(struct_size, 0);
RUSTERON_DPDK_CONFIG_AT(file_prefix, 4);
RUSTERON_DPDK_CONFIG_AT(hugepage_dir, 69);
RUSTERON_DPDK_CONFIG_AT(sender_pci, 4165);
RUSTERON_DPDK_CONFIG_AT(sender_ipv4, 4181);
RUSTERON_DPDK_CONFIG_AT(sender_prefix_len, 4197);
RUSTERON_DPDK_CONFIG_AT(sender_gateway, 4198);
RUSTERON_DPDK_CONFIG_AT(receiver_pci, 4214);
RUSTERON_DPDK_CONFIG_AT(receiver_ipv4, 4230);
RUSTERON_DPDK_CONFIG_AT(receiver_prefix_len, 4246);
RUSTERON_DPDK_CONFIG_AT(receiver_gateway, 4247);
RUSTERON_DPDK_CONFIG_AT(rx_descriptors, 4264);
RUSTERON_DPDK_CONFIG_AT(tx_descriptors, 4266);
RUSTERON_DPDK_CONFIG_AT(mbufs_per_port, 4268);
RUSTERON_DPDK_CONFIG_AT(mempool_cache, 4272);
RUSTERON_DPDK_CONFIG_AT(burst_size, 4274);
RUSTERON_DPDK_CONFIG_AT(max_aeron_mtu, 4280);
RUSTERON_DPDK_CONFIG_AT(sender_mac, 4288);
RUSTERON_DPDK_CONFIG_AT(receiver_mac, 4306);
RUSTERON_DPDK_CONFIG_AT(rx_hw_timestamp, 4324);
RUSTERON_DPDK_CONFIG_AT(clock, 4325);
_Static_assert(sizeof(rusteron_dpdk_config_t) == 4328, "config size changed: update the Rust mirrors");
/* The clock report, field by field (src/ffi.rs asserts the Rust side). */
#define RUSTERON_DPDK_CLOCK_REPORT_AT(field, offset) \
    _Static_assert(offsetof(rusteron_dpdk_clock_report_t, field) == (offset), #field " moved: update src/ffi.rs")
RUSTERON_DPDK_CLOCK_REPORT_AT(hz, 0);
RUSTERON_DPDK_CLOCK_REPORT_AT(eal_hz, 8);
RUSTERON_DPDK_CLOCK_REPORT_AT(cntfrq, 16);
RUSTERON_DPDK_CLOCK_REPORT_AT(source, 24);
RUSTERON_DPDK_CLOCK_REPORT_AT(why, 28);
RUSTERON_DPDK_CLOCK_REPORT_AT(invariant, 32);
RUSTERON_DPDK_CLOCK_REPORT_AT(reserved, 36);
_Static_assert(sizeof(rusteron_dpdk_clock_report_t) == 40, "clock report size changed: update src/ffi.rs");

/* The threading modes the runtime keys housekeeping and pacing on are Aeron's. */
_Static_assert(RUSTERON_DPDK_THREADING_DEDICATED == (int)AERON_THREADING_MODE_DEDICATED, "Aeron's DEDICATED moved");
_Static_assert(RUSTERON_DPDK_THREADING_SHARED_NETWORK == (int)AERON_THREADING_MODE_SHARED_NETWORK,
               "Aeron's SHARED_NETWORK moved");
_Static_assert(RUSTERON_DPDK_THREADING_SHARED == (int)AERON_THREADING_MODE_SHARED, "Aeron's SHARED moved");
_Static_assert(RUSTERON_DPDK_THREADING_INVOKER == (int)AERON_THREADING_MODE_INVOKER, "Aeron's INVOKER moved");

static int rusteron_dpdk_config_validate(const rusteron_dpdk_config_t *config)
{
    if (NULL == config)
    {
        rusteron_dpdk_set_error("config must not be NULL");
        return -1;
    }

    if (config->struct_size != RUSTERON_DPDK_CONFIG_STRUCT_SIZE)
    {
        rusteron_dpdk_set_error("config.struct_size does not match the native layout — Rust/native ABI mismatch");
        return -1;
    }

    if (rusteron_dpdk_config_validate_string(config->file_prefix, sizeof(config->file_prefix), "file_prefix") < 0 ||
        rusteron_dpdk_config_validate_string(config->hugepage_dir, sizeof(config->hugepage_dir), "hugepage_dir") < 0 ||
        rusteron_dpdk_config_validate_string(config->sender_pci, sizeof(config->sender_pci), "sender_pci") < 0 ||
        rusteron_dpdk_config_validate_string(config->sender_ipv4, sizeof(config->sender_ipv4), "sender_ipv4") < 0 ||
        rusteron_dpdk_config_validate_string(config->sender_gateway, sizeof(config->sender_gateway), "sender_gateway") < 0 ||
        rusteron_dpdk_config_validate_string(config->receiver_pci, sizeof(config->receiver_pci), "receiver_pci") < 0 ||
        rusteron_dpdk_config_validate_string(config->receiver_ipv4, sizeof(config->receiver_ipv4), "receiver_ipv4") < 0 ||
        rusteron_dpdk_config_validate_string(config->receiver_gateway, sizeof(config->receiver_gateway), "receiver_gateway") < 0)
    {
        return -1;
    }

    const char *macs[2] = { config->sender_mac, config->receiver_mac };
    const char *mac_names[2] = { "sender_mac", "receiver_mac" };
    for (int i = 0; i < 2; i++)
    {
        uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN];
        if (memchr(macs[i], '\0', sizeof(config->sender_mac)) == NULL ||
            ('\0' != macs[i][0] && rusteron_dpdk_parse_mac(macs[i], mac) < 0))
        {
            char message[256];
            snprintf(message, sizeof(message),
                     "config field '%s' must be empty or a MAC aa:bb:cc:dd:ee:ff", mac_names[i]);
            rusteron_dpdk_set_error(message);
            return -1;
        }
    }

    if (config->sender_prefix_len < 1 || config->sender_prefix_len > 32 ||
        config->receiver_prefix_len < 1 || config->receiver_prefix_len > 32)
    {
        rusteron_dpdk_set_error("prefix lengths must be in 1..=32");
        return -1;
    }

    if (config->rx_hw_timestamp > 1)
    {
        rusteron_dpdk_set_error("rx_hw_timestamp must be 0 or 1");
        return -1;
    }
    if (config->burst_size < 1 || config->burst_size > 256)
    {
        rusteron_dpdk_set_error("burst_size must be in 1..=256");
        return -1;
    }

    /* rx_descriptors 0: the NIC's default, sized per port at start. */
    if ((0 != config->rx_descriptors && (config->rx_descriptors < 64 || config->rx_descriptors > 8192)) ||
        config->tx_descriptors < 64 || config->tx_descriptors > 8192)
    {
        rusteron_dpdk_set_error("rx/tx descriptors must be in 64..=8192 (rx 0: the NIC's default)");
        return -1;
    }

    if (config->max_aeron_mtu == 0 || config->max_aeron_mtu > RUSTERON_DPDK_MAX_AERON_MTU || (config->max_aeron_mtu % 32) != 0)
    {
        rusteron_dpdk_set_error("max_aeron_mtu must be non-zero, 32-aligned, and <= 8960 (and fit the NIC's MTU)");
        return -1;
    }

    if (config->mbufs_per_port == 0)
    {
        rusteron_dpdk_set_error("mbufs_per_port must be non-zero");
        return -1;
    }

    if (RUSTERON_DPDK_CLOCK_CHOICE_COUNTER != config->clock && RUSTERON_DPDK_CLOCK_CHOICE_MONOTONIC != config->clock)
    {
        rusteron_dpdk_set_error("clock must be 0 (the counter) or 1 (CLOCK_MONOTONIC)");
        return -1;
    }

    return 0;
}

int rusteron_dpdk_transport_create(
    const rusteron_dpdk_config_t *config,
    rusteron_dpdk_transport_t **transport)
{
    if (NULL == transport)
    {
        rusteron_dpdk_set_error("transport out-param must not be NULL");
        return -1;
    }

    *transport = NULL;

    if (rusteron_dpdk_config_validate(config) < 0)
    {
        return -1;
    }

    /* Cache-line aligned: its readiness flags each own a line. */
    const size_t native_size = (sizeof(rusteron_dpdk_transport_t) + 63) & ~(size_t)63;
    rusteron_dpdk_transport_t *native = aligned_alloc(64, native_size);
    if (NULL == native)
    {
        rusteron_dpdk_set_error("out of memory allocating DPDK transport");
        return -1;
    }
    memset(native, 0, native_size);

    native->config = *config;

    if (rusteron_dpdk_runtime_init(native) < 0)
    {
        if (native->housekeeping_lock_ready)
        {
            pthread_mutex_destroy(&native->housekeeping_lock);
            pthread_mutex_destroy(&native->port_table_lock);
        }
        free(native->host_ips);
        free(native);
        return -1;
    }

    rusteron_dpdk_the_runtime = native;
    *transport = native;
    return 0;
}

int rusteron_dpdk_transport_install(
    rusteron_dpdk_transport_t *transport,
    aeron_driver_context_t *context)
{
    if (NULL == transport)
    {
        rusteron_dpdk_set_error("transport must not be NULL");
        return -1;
    }

    if (NULL == context)
    {
        rusteron_dpdk_set_error("context must not be NULL");
        return -1;
    }

    if (rusteron_dpdk_runtime_agents_known(
            transport, (int)context->threading_mode, context->conductor_cpu_affinity_no,
            context->sender_cpu_affinity_no, context->receiver_cpu_affinity_no,
            context->conductor_idle_strategy_name) < 0)
    {
        return -1;
    }
    /* Only the sender/receiver bindings are replaced; the conductor's
     * resolver bindings stay on the socket media so DNS resolution keeps
     * working over the kernel. */
    context->udp_channel_transport_bindings = rusteron_dpdk_transport_bindings();
    return 0;
}

int rusteron_dpdk_transport_close(rusteron_dpdk_transport_t *transport)
{
    if (NULL == transport)
    {
        return 0;
    }

    if (rusteron_dpdk_the_runtime == transport)
    {
        rusteron_dpdk_the_runtime = NULL;
    }
    rusteron_dpdk_runtime_cleanup(transport);
    if (transport->housekeeping_lock_ready)
    {
        pthread_mutex_destroy(&transport->housekeeping_lock);
        pthread_mutex_destroy(&transport->port_table_lock);
    }
    free(transport);
    return 0;
}

/* Test reset: drop the singleton so a fresh transport can be created in the
 * next test after the EAL guard was cleared by rusteron_dpdk_test_reset(). */
void rusteron_dpdk_transport_test_reset(void)
{
    rusteron_dpdk_the_runtime = NULL;
}

/* --- Aeron transport binding callbacks ------------------------------------ */

static int rusteron_dpdk_transport_init(
    aeron_udp_channel_transport_t *transport,
    struct sockaddr_storage *bind_addr,
    struct sockaddr_storage *multicast_if_addr,
    struct sockaddr_storage *connect_addr,
    aeron_udp_channel_transport_params_t *params,
    aeron_driver_context_t *context,
    aeron_udp_channel_transport_affinity_t affinity)
{
    (void)multicast_if_addr;

    if (NULL == bind_addr)
    {
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport init: bind_addr is NULL");
        rusteron_dpdk_set_error("DPDK transport init: bind_addr must not be NULL");
        return -1;
    }
    if (AF_INET != bind_addr->ss_family)
    {
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport supports IPv4 unicast only");
        rusteron_dpdk_set_error("DPDK transport init: only IPv4 bind addresses are supported");
        return -1;
    }
    if (NULL == params || 0 == params->mtu_length)
    {
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport init: mtu_length must be greater than 0");
        rusteron_dpdk_set_error("DPDK transport init: mtu_length must be greater than 0");
        return -1;
    }

    rusteron_dpdk_transport_t *native = rusteron_dpdk_active_runtime();
    if (NULL == native)
    {
        AERON_SET_ERR(ENODEV, "%s", "DPDK transport init: no native runtime (rusteron_dpdk_transport_create was not called)");
        rusteron_dpdk_set_error("DPDK transport init: no native runtime");
        return -1;
    }


    rusteron_dpdk_port_t *port;
    rusteron_dpdk_port_table_t *table;
    switch (affinity)
    {
        case AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER:
            port = &native->sender;
            table = &native->sender_ports;
            break;
        case AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER:
            port = &native->receiver;
            table = &native->receiver_ports;
            break;
        default:
            AERON_SET_ERR(EINVAL, "DPDK transport init: unsupported affinity %d", (int)affinity);
            rusteron_dpdk_set_error("DPDK transport init: conductor affinity is not a data-path transport");
            return -1;
    }

    const struct sockaddr_in *bind_in = (const struct sockaddr_in *)bind_addr;
    const struct sockaddr_in *connect_in =
        (NULL != connect_addr && AF_INET == connect_addr->ss_family) ? (const struct sockaddr_in *)connect_addr : NULL;
    const uint32_t bind_ip = bind_in->sin_addr.s_addr;
    const uint16_t bind_port = (uint16_t)ntohs(bind_in->sin_port);

    if (rusteron_dpdk_ipv4_is_multicast(bind_ip) ||
        (NULL != connect_in && rusteron_dpdk_ipv4_is_multicast(connect_in->sin_addr.s_addr)))
    {
        AERON_SET_ERR(EPROTONOSUPPORT, "%s", "DPDK transport: multicast channels are not supported (use unicast or MDC)");
        rusteron_dpdk_set_error("DPDK transport init: multicast channels are not supported");
        return -1;
    }
    if (NULL != connect_in &&
        (INADDR_ANY == connect_in->sin_addr.s_addr || INADDR_BROADCAST == connect_in->sin_addr.s_addr ||
         0 == connect_in->sin_port))
    {
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport init: cannot connect to 0.0.0.0, broadcast or port 0");
        rusteron_dpdk_set_error("DPDK transport init: invalid connect address");
        return -1;
    }

    /* Host-local channels (loopback, the host's kernel interfaces) cannot be
     * reached through a DPDK NIC: they go through a kernel socket, exactly as
     * Aeron's own kernel transport would carry them. */
    if (rusteron_dpdk_is_host_local(native, bind_ip) ||
        (NULL != connect_in && rusteron_dpdk_is_host_local(native, connect_in->sin_addr.s_addr)))
    {
        return rusteron_dpdk_transport_init_delegated(
            transport, native, port, bind_in, connect_in, connect_addr, params, context);
    }

    /* Channel-specific MTU overrides larger than max_aeron_mtu are rejected at
     * init (plan §6.5) rather than clamped, so an over-large channel fails
     * loudly instead of silently degrading to a different MTU. Host-local
     * channels above have no such limit (the kernel carries up to 64 KiB). */
    if (params->mtu_length > native->config.max_aeron_mtu)
    {
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport init: channel MTU exceeds max_aeron_mtu");
        rusteron_dpdk_set_error("DPDK transport init: channel MTU exceeds max_aeron_mtu");
        return -1;
    }

    if (!port->link_up)
    {
        AERON_SET_ERR(ENODEV, "DPDK transport init: role ENA (port %u) is not up", port->port_id);
        rusteron_dpdk_set_error("DPDK transport init: role ENA is not up");
        return -1;
    }

    /* A kernel socket can bind only addresses the host owns: here that is
     * this NIC's (its own or an extra one) or the wildcard, narrowed to it. */
    const int bound_index = rusteron_dpdk_ip_index(port, bind_ip);
    if (bound_index < 0)
    {
        char ip_text[INET_ADDRSTRLEN] = "";
        inet_ntop(AF_INET, &bind_in->sin_addr, ip_text, sizeof(ip_text));
        AERON_SET_ERR(EADDRNOTAVAIL,
                      "DPDK transport init: %s is not the %s NIC's address %s (bind 0.0.0.0, the NIC's IP, or a "
                      "host-local address)",
                      ip_text, RUSTERON_DPDK_ROLE_SENDER == port->role ? "sender" : "receiver", port->local_ipv4);
        rusteron_dpdk_set_error("DPDK transport init: bind address is not this NIC's");
        return -1;
    }

    /* A wildcard receive bind also listens on the host's own interfaces
     * (loopback, kernel IPs), so same-host publishers reach it as they would
     * a kernel socket; RUSTERON_DPDK_WILDCARD_LOCAL=0 opts out. Its socket is
     * read only when the local-I/O thread finds data waiting. */
    const int listen_local =
        native->wildcard_local && AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER == affinity && INADDR_ANY == bind_ip;

    /* Claim the UDP port exclusively, as a kernel bind() would: an explicit
     * port in use is EADDRINUSE now, not a dead channel later; port 0 gets
     * an ephemeral port, and one the host's kernel already uses (for the
     * local listener) is skipped like a bound one. Port 0 never reaches the
     * wire. Cold path: the kernel bind below runs under the table lock. */
    int kernel_fd = -1;
    uint16_t local_port = 0;
    uint16_t tried[64];
    size_t tried_count = 0;
    pthread_mutex_lock(&native->port_table_lock);
    for (;;)
    {
        if (0 != bind_port)
        {
            if (rusteron_dpdk_port_bound(table, bound_index, bind_port))
            {
                pthread_mutex_unlock(&native->port_table_lock);
                AERON_SET_ERR(EADDRINUSE, "DPDK transport init: UDP port %u is already bound on %s",
                              (unsigned)bind_port, port->local_ipv4);
                rusteron_dpdk_set_error("DPDK transport init: UDP port already bound");
                return -1;
            }
            local_port = bind_port;
        }
        else
        {
            local_port = rusteron_dpdk_ephemeral_port(native, table, context);
            if (0 == local_port)
            {
                for (size_t t = 0; t < tried_count; t++)
                {
                    rusteron_dpdk_port_set_bound(table, 0, tried[t], 0);
                }
                pthread_mutex_unlock(&native->port_table_lock);
                AERON_SET_ERR(EADDRINUSE, "DPDK transport init: no free ephemeral UDP port in %u-%u on %s",
                              (unsigned)native->ephemeral_low, (unsigned)native->ephemeral_high, port->local_ipv4);
                rusteron_dpdk_set_error("DPDK transport init: no free ephemeral UDP port");
                return -1;
            }
        }
        if (!listen_local)
        {
            break;
        }
        struct sockaddr_in any = { .sin_family = AF_INET, .sin_port = htons(local_port) };
        uint16_t got = 0;
        if (0 == rusteron_dpdk_kernel_open(&kernel_fd, &any, NULL, params, &got))
        {
            break;
        }
        if (0 != bind_port || EADDRINUSE != aeron_errcode() || tried_count == sizeof(tried) / sizeof(tried[0]))
        {
            for (size_t t = 0; t < tried_count; t++)
            {
                rusteron_dpdk_port_set_bound(table, 0, tried[t], 0);
            }
            pthread_mutex_unlock(&native->port_table_lock);
            rusteron_dpdk_set_error("DPDK transport init: cannot open the wildcard-local kernel socket");
            return -1;
        }
        /* The host already uses that port: skip it, as the kernel's own
         * ephemeral pick would, and try the next. */
        rusteron_dpdk_port_set_bound(table, 0, local_port, 1);
        tried[tried_count++] = local_port;
    }
    for (size_t t = 0; t < tried_count; t++)
    {
        rusteron_dpdk_port_set_bound(table, 0, tried[t], 0);
    }
    rusteron_dpdk_port_set_bound(table, bound_index, local_port, 1);
    pthread_mutex_unlock(&native->port_table_lock);

    rusteron_dpdk_client_t *client = rusteron_dpdk_client_alloc();
    if (NULL == client)
    {
        rusteron_dpdk_kernel_close(&kernel_fd);
        rusteron_dpdk_port_release(native, table, bound_index, local_port);
        AERON_SET_ERR(ENOMEM, "%s", "DPDK transport init: out of memory allocating client state");
        rusteron_dpdk_set_error("DPDK transport init: out of memory allocating client state");
        return -1;
    }
    client->runtime = native;
    client->port = port;
    client->affinity = (int)affinity;
    client->local_udp_port = local_port;
    client->bound_ip = bind_ip;
    client->bound_index = bound_index;
    client->kernel_fd = kernel_fd;
    client->ksock = -1;
    client->lazy_fd = -1;
    client->lazy_ksock = -1;
    client->route_index = -2;
    /* Oversized channel MTUs were rejected earlier in this function, so the
     * client MTU is always <= max_aeron_mtu and needs no clamp. */
    client->mtu = params->mtu_length;
    client->params = *params;
    if (kernel_fd >= 0)
    {
        client->ksock = rusteron_dpdk_ksock_register(native, kernel_fd);
        if (client->ksock < 0)
        {
            rusteron_dpdk_kernel_close(&client->kernel_fd);
            free(client);
            rusteron_dpdk_port_release(native, table, bound_index, local_port);
            return -1;
        }
    }

    /* Surface the port's counters into the driver's counters manager (plan §9).
     * Best effort: a NULL context (unit tests) or a NULL counters manager skips
     * registration and the counters bumps become no-ops. Nothing can fail
     * after this, so the open-transports count never leaks. */
    pthread_mutex_lock(&native->housekeeping_lock);
    rusteron_dpdk_counters_register(
        &port->counters,
        NULL != context ? context->counters_manager : NULL,
        port, native->ops);
    port->open_transports++;
    if (NULL != context && NULL != context->error_log)
    {
        port->error_log = context->error_log; /* housekeeping's reports, while a transport is open */
    }
    pthread_mutex_unlock(&native->housekeeping_lock);
    pthread_mutex_lock(&native->port_table_lock);
    client->lazy_next = native->lazy_clients;
    if (NULL != native->lazy_clients)
    {
        native->lazy_clients->lazy_prev = client;
    }
    native->lazy_clients = client;
    client->lazy_linked = 1;
    pthread_mutex_unlock(&native->port_table_lock);
    rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_TRANSPORT, 1);
    /* Its NIC's polls stamp each burst for it while it is open. */
    if (params->is_media_timestamping)
    {
        client->media_ts = 1;
        __atomic_add_fetch(&port->media_ts_transports, 1, __ATOMIC_RELAXED);
    }

    rusteron_dpdk_transport_init_fields(transport, context, params);
    transport->bindings_clientd = client;
    /* The connect address is caller-owned; keep a copy so NULL-address sends
     * survive past the caller's frame (connected transport, plan §7.4). A
     * connected transport, like a connect()ed socket, hears only its peer. */
    transport->connected_address = NULL;
    if (NULL != connect_in)
    {
        memcpy(&client->connected_address, connect_addr, sizeof(client->connected_address));
        transport->connected_address = &client->connected_address;
        client->connected = 1;
        client->connected_ip = connect_in->sin_addr.s_addr;
        client->connected_port = (uint16_t)ntohs(connect_in->sin_port);
    }
    transport->fd = -1; /* the DPDK transport owns no socket */
    transport->recv_fd = -1;

    return 0;
}

/* Whether binding `port` on row `index` (0 wildcard, 1 NIC, 2+ extra IP)
 * conflicts, as a kernel bind() would decide. */
static int rusteron_dpdk_port_bound(const rusteron_dpdk_port_table_t *table, int index, uint16_t port)
{
    const uint64_t bit = 1ULL << (port % 64);
    if (0 != (table->bound[0][port / 64] & bit))
    {
        return 1;
    }
    if (0 != index)
    {
        return 0 != (table->bound[index][port / 64] & bit);
    }
    for (int i = 1; i < 2 + RUSTERON_DPDK_MAX_EXTRA_IPS; i++)
    {
        if (0 != (table->bound[i][port / 64] & bit))
        {
            return 1;
        }
    }
    return 0;
}

static void rusteron_dpdk_port_set_bound(rusteron_dpdk_port_table_t *table, int index, uint16_t port, int bound)
{
    if (bound)
    {
        table->bound[index][port / 64] |= 1ULL << (port % 64);
    }
    else
    {
        table->bound[index][port / 64] &= ~(1ULL << (port % 64));
    }
}

static void rusteron_dpdk_port_release(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_table_t *table, int index, uint16_t port)
{
    pthread_mutex_lock(&native->port_table_lock);
    rusteron_dpdk_port_set_bound(table, index, port, 0);
    pthread_mutex_unlock(&native->port_table_lock);
}

/* The port-table row of `ip` on `port_info`: 0 wildcard, 1 the NIC's own, 2+i
 * extra address i; -1 when the NIC does not own it. */
static int rusteron_dpdk_ip_index(const rusteron_dpdk_port_t *port_info, uint32_t ip)
{
    if (INADDR_ANY == ip)
    {
        return 0;
    }
    if (ip == port_info->local_ip)
    {
        return 1;
    }
    for (uint32_t i = 0; i < port_info->extra_ip_count; i++)
    {
        if (port_info->extra_ips[i] == ip)
        {
            return 2 + (int)i;
        }
    }
    return -1;
}

/* Whether Aeron's configured wildcard port ranges (which it hands out itself,
 * for receivers and control-address senders) include `port`. */
static int rusteron_dpdk_in_aeron_range(const aeron_driver_context_t *context, uint16_t port)
{
    if (NULL == context)
    {
        return 0;
    }
    const aeron_wildcard_port_manager_t *ranges[2] = {
        &context->sender_wildcard_port_manager, &context->receiver_wildcard_port_manager };
    for (int i = 0; i < 2; i++)
    {
        if (!ranges[i]->is_os_wildcard && ranges[i]->low_port <= port && port <= ranges[i]->high_port)
        {
            return 1;
        }
    }
    return 0;
}

/* The next free ephemeral port on the NIC, as the kernel would pick one: from
 * the resolved range, starting where the last search stopped (randomly at
 * first), skipping bound ports, ip_local_reserved_ports and Aeron's own
 * ranges. 0 when none is free. Caller holds port_table_lock. */
static uint16_t rusteron_dpdk_ephemeral_port(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_table_t *table, const aeron_driver_context_t *context)
{
    const uint32_t low = native->ephemeral_low, high = native->ephemeral_high;
    const uint32_t span = high - low + 1;
    uint32_t start = (table->ephemeral_next >= low && table->ephemeral_next <= high) ? (uint32_t)table->ephemeral_next - low : 0u;
    for (uint32_t i = 0; i < span; i++)
    {
        const uint16_t candidate = (uint16_t)(low + (start + i) % span);
        if (!rusteron_dpdk_port_bound(table, 0, candidate) &&
            0 == (native->reserved_ports[candidate / 64] & (1ULL << (candidate % 64))) &&
            !rusteron_dpdk_in_aeron_range(context, candidate))
        {
            table->ephemeral_next = (uint16_t)(candidate == high ? low : (uint32_t)candidate + 1u);
            return candidate;
        }
    }
    return 0;
}

/* The fields Aeron's kernel transport initialises (aeron_udp_channel_transport.c
 * init): error log and counter from the context, no interceptors yet, and
 * media receive timestamps when the channel asks for them
 * (media-rcv-ts-offset): Aeron's image then writes each data frame's receive
 * time at the offset, so every datagram such a transport receives comes with
 * one (the poller's, or the kernel socket's SO_TIMESTAMPNS). */
static void rusteron_dpdk_transport_init_fields(
    aeron_udp_channel_transport_t *transport, aeron_driver_context_t *context,
    const aeron_udp_channel_transport_params_t *params)
{
    transport->error_log = NULL != context ? context->error_log : NULL;
    transport->errors_counter = NULL != context && NULL != context->system_counters
        ? aeron_system_counter_addr(context->system_counters, AERON_SYSTEM_COUNTER_ERRORS)
        : NULL;
    transport->timestamp_flags = params->is_media_timestamping ? AERON_UDP_CHANNEL_TRANSPORT_MEDIA_RCV_TIMESTAMP
                                                               : AERON_UDP_CHANNEL_TRANSPORT_MEDIA_RCV_TIMESTAMP_NONE;
    for (size_t i = 0; i < AERON_UDP_CHANNEL_TRANSPORT_MAX_INTERCEPTORS; i++)
    {
        transport->interceptor_clientds[i] = NULL;
    }
}

/* A host-local channel: the whole transport runs on a kernel socket. */
static int rusteron_dpdk_transport_init_delegated(
    aeron_udp_channel_transport_t *transport,
    rusteron_dpdk_transport_t *native,
    rusteron_dpdk_port_t *port,
    const struct sockaddr_in *bind_in,
    const struct sockaddr_in *connect_in,
    struct sockaddr_storage *connect_addr,
    aeron_udp_channel_transport_params_t *params,
    aeron_driver_context_t *context)
{
    rusteron_dpdk_client_t *client = rusteron_dpdk_client_alloc();
    if (NULL == client)
    {
        AERON_SET_ERR(ENOMEM, "%s", "DPDK transport init: out of memory allocating client state");
        rusteron_dpdk_set_error("DPDK transport init: out of memory allocating client state");
        return -1;
    }
    client->runtime = native;
    client->port = port;
    client->delegated = 1;
    client->kernel_fd = -1;
    client->ksock = -1;
    client->lazy_fd = -1;
    client->lazy_ksock = -1;
    client->mtu = params->mtu_length;
    client->params = *params;
    if (rusteron_dpdk_kernel_open(&client->kernel_fd, bind_in, connect_in, params, &client->local_udp_port) < 0)
    {
        rusteron_dpdk_set_error("DPDK transport init: cannot open the host-local kernel socket");
        free(client);
        return -1;
    }
    client->ksock = rusteron_dpdk_ksock_register(native, client->kernel_fd);
    if (client->ksock < 0)
    {
        rusteron_dpdk_kernel_close(&client->kernel_fd);
        free(client);
        return -1;
    }
    /* The port's counters too, as a DPDK channel registers them: its agent
     * polls the port either way, and on localhost only these channels run,
     * so its loop gaps would otherwise go uncounted. */
    pthread_mutex_lock(&native->housekeeping_lock);
    rusteron_dpdk_counters_register(
        &port->counters,
        NULL != context ? context->counters_manager : NULL,
        port, native->ops);
    port->open_transports++;
    if (NULL != context && NULL != context->error_log)
    {
        port->error_log = context->error_log; /* housekeeping's reports, while a transport is open */
    }
    pthread_mutex_unlock(&native->housekeeping_lock);
    rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_TRANSPORT, 1);
    rusteron_dpdk_transport_init_fields(transport, context, params);
    transport->bindings_clientd = client;
    transport->connected_address = NULL;
    if (NULL != connect_in)
    {
        memcpy(&client->connected_address, connect_addr, sizeof(client->connected_address));
        transport->connected_address = &client->connected_address;
        client->connected = 1;
    }
    transport->fd = client->kernel_fd;
    transport->recv_fd = client->kernel_fd;
    return 0;
}

static int rusteron_dpdk_transport_reconnect(
    aeron_udp_channel_transport_t *transport,
    struct sockaddr_storage *connect_addr)
{
    rusteron_dpdk_client_t *client = transport->bindings_clientd;
    /* As the kernel transport: only an already-connected transport reconnects. */
    if (NULL == connect_addr || NULL == client || NULL == transport->connected_address)
    {
        return NULL == client && NULL != connect_addr ? -1 : 0;
    }
    if (AF_INET != connect_addr->ss_family)
    {
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport reconnect: only IPv4 is supported");
        return -1;
    }
    const struct sockaddr_in *in = (const struct sockaddr_in *)connect_addr;
    if (client->delegated && connect(client->kernel_fd, (const struct sockaddr *)in, sizeof(*in)) < 0)
    {
        AERON_SET_ERR(errno, "%s", "DPDK transport reconnect: connect");
        return -1;
    }
    memcpy(&client->connected_address, connect_addr, sizeof(client->connected_address));
    transport->connected_address = &client->connected_address;
    client->connected_ip = in->sin_addr.s_addr;
    client->connected_port = (uint16_t)ntohs(in->sin_port);
    return 0;
}

static int rusteron_dpdk_transport_close_transport(aeron_udp_channel_transport_t *transport)
{
    if (NULL == transport)
    {
        return 0;
    }
    rusteron_dpdk_client_t *client = transport->bindings_clientd;
    if (NULL != client)
    {
        rusteron_dpdk_transport_t *runtime = client->runtime;
        if (client->lazy_linked)
        {
            /* Off the list first: the local-I/O thread opens under this lock,
             * so it is done with this client once unlinked. */
            pthread_mutex_lock(&runtime->port_table_lock);
            if (NULL != client->lazy_prev)
            {
                client->lazy_prev->lazy_next = client->lazy_next;
            }
            else
            {
                runtime->lazy_clients = client->lazy_next;
            }
            if (NULL != client->lazy_next)
            {
                client->lazy_next->lazy_prev = client->lazy_prev;
            }
            client->lazy_linked = 0;
            pthread_mutex_unlock(&runtime->port_table_lock);
        }
        /* Opened for it but never adopted by its agent. */
        rusteron_dpdk_ksock_unregister(runtime, client->lazy_ksock, client->lazy_fd);
        rusteron_dpdk_kernel_close(&client->lazy_fd);
        rusteron_dpdk_ksock_unregister(client->runtime, client->ksock, client->kernel_fd);
        client->ksock = -1;
        rusteron_dpdk_kernel_close(&client->kernel_fd);
        if (NULL != client->port)
        {
            rusteron_dpdk_transport_t *native = client->runtime;
            rusteron_dpdk_counters_add(&client->port->counters, RD_COUNTER_TRANSPORT, -1);
            if (client->media_ts)
            {
                __atomic_sub_fetch(&client->port->media_ts_transports, 1, __ATOMIC_RELAXED);
            }
            /* The port's last transport: its counters stay allocated but
             * dormant, never written (no transports, no sampling), so they
             * keep reading their totals between channels, as Aeron's own
             * counters do. The next transport on the port reuses them if the
             * manager still holds them under their token, else registers
             * afresh (a new driver's manager). Nothing here touches the
             * manager, which is the conductor's alone. */
            pthread_mutex_lock(&native->housekeeping_lock);
            const int last = --client->port->open_transports <= 0;
            if (last)
            {
                client->port->open_transports = 0;
                client->port->counters.dormant = 1;
            }
            pthread_mutex_unlock(&native->housekeeping_lock);
            /* nothing will send or poll this NIC again: its queued frames go */
            if (last)
            {
                rusteron_dpdk_port_tx_drop_queued(native, client->port);
            }
            /* a host-local channel holds no DPDK port number */
            if (!client->delegated)
            {
                rusteron_dpdk_port_release(
                    native,
                    client->port == &native->sender ? &native->sender_ports : &native->receiver_ports,
                    client->bound_index,
                    client->local_udp_port);
            }
        }
    }
    transport->bindings_clientd = NULL;
    transport->connected_address = NULL;
    transport->fd = -1;
    transport->recv_fd = -1;
    free(client);
    return 0;
}

/* Aeron drives receives through the poller; this binding exists for callers
 * that receive on one transport. It never drops another endpoint's frames: a
 * transport registered with a poller receives through the poller's full
 * endpoint map, as poller_poll would; an unregistered one is the only endpoint
 * it can dispatch to. A host-local channel reads its kernel socket. */
static int rusteron_dpdk_transport_recvmmsg(
    aeron_udp_channel_transport_t *transport,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd)
{
    rusteron_dpdk_client_t *client = transport != NULL ? transport->bindings_clientd : NULL;
    if (NULL == client)
    {
        return 0;
    }
    if (client->delegated)
    {
        const int received = rusteron_dpdk_kernel_recv(
            client->kernel_fd, NULL, transport, msgvec, vlen, bytes_rcved, recv_func, clientd);
        return RUSTERON_DPDK_KERNEL_RECV_AGAIN == received ? 0 : received;
    }
    if (NULL != client->poller)
    {
        return rusteron_dpdk_poller_receive_registered(client, msgvec, vlen, bytes_rcved, recv_func, clientd);
    }
    return rusteron_dpdk_poller_receive(
        client, transport, NULL, msgvec, vlen, bytes_rcved, recv_func, clientd);
}

/* A DPDK transport's host-local destination: through a kernel socket, put
 * on the transport's poller so replies are received. The agent never opens
 * it: the first send asks the local-I/O thread (kernel.c) and reports a zero
 * send, which Aeron retries, until the socket is ready (within ~10 ms). A
 * failed open is reported once and the datagrams dropped, as a failed
 * neighbour's are, while the local-I/O thread retries it each second. */
/* Aeron frame types (aeron_udp_protocol.h), as the poller reads SETUP. */
#define RUSTERON_DPDK_AERON_HDR_TYPE_PAD 0x00
#define RUSTERON_DPDK_AERON_HDR_TYPE_DATA 0x01

/* Whether a send is of a low-priority stream (RUSTERON_DPDK_TX_LOW_STREAMS):
 * a data or padding frame whose stream id (offset 16, little-endian as Aeron
 * writes it) is listed. A send is one publication's frames, so the first
 * tells. Control frames (status messages, NAKs, setups) never are. */
static inline int rusteron_dpdk_tx_low(
    const rusteron_dpdk_transport_t *native, const struct iovec *iov, size_t iov_length)
{
    if (0 == native->tx_low_count || 0 == iov_length || iov[0].iov_len < 20)
    {
        return 0;
    }
    const uint8_t *f = (const uint8_t *)iov[0].iov_base;
    const uint16_t type = (uint16_t)(f[6] | (f[7] << 8));
    if (RUSTERON_DPDK_AERON_HDR_TYPE_DATA != type && RUSTERON_DPDK_AERON_HDR_TYPE_PAD != type)
    {
        return 0;
    }
    const int32_t stream =
        (int32_t)((uint32_t)f[16] | ((uint32_t)f[17] << 8) | ((uint32_t)f[18] << 16) | ((uint32_t)f[19] << 24));
    for (uint32_t i = 0; i < native->tx_low_count; i++)
    {
        if (stream >= native->tx_low_first[i] && stream <= native->tx_low_last[i])
        {
            return 1;
        }
    }
    return 0;
}

static int rusteron_dpdk_transport_send_local(
    aeron_udp_channel_transport_t *transport,
    rusteron_dpdk_client_t *client,
    const struct sockaddr_storage *dst,
    struct iovec *iov,
    size_t iov_length,
    int64_t *bytes_sent)
{
    if (client->kernel_fd < 0)
    {
        switch (__atomic_load_n(&client->lazy_state, __ATOMIC_ACQUIRE))
        {
            case RUSTERON_DPDK_LAZY_NONE:
                client->lazy_dst_ip = ((const struct sockaddr_in *)dst)->sin_addr.s_addr;
                __atomic_store_n(&client->lazy_state, RUSTERON_DPDK_LAZY_REQUESTED, __ATOMIC_RELEASE);
                return 0;
            case RUSTERON_DPDK_LAZY_REQUESTED:
                return 0;
            case RUSTERON_DPDK_LAZY_FAILED:
                if (!client->lazy_reported)
                {
                    client->lazy_reported = 1;
                    AERON_SET_ERR(EAGAIN, "%s", "DPDK transport send: cannot open the host-local kernel socket");
                    rusteron_dpdk_set_error("DPDK transport send: cannot open the host-local kernel socket");
                    return -1;
                }
                for (size_t i = 0; i < iov_length; i++)
                {
                    *bytes_sent += (int64_t)iov[i].iov_len;
                }
                return (int)iov_length;
            default: /* READY: adopt it */
                client->kernel_fd = client->lazy_fd;
                client->ksock = client->lazy_ksock;
                client->lazy_fd = -1;
                client->lazy_ksock = -1;
                client->lazy_reported = 0;
                break;
        }
    }
    if (!client->kernel_polled && NULL != client->poller)
    {
        /* Its list has room for every transport's socket (poller_add). */
        client->kernel_polled = rusteron_dpdk_poller_kernel_add(client->poller, transport) == 0;
    }
    int rc = rusteron_dpdk_kernel_send(client->kernel_fd, dst, iov, iov_length, bytes_sent);
    if (rc < 0)
    {
        rusteron_dpdk_set_error("DPDK transport send: host-local kernel send failed");
    }
    return rc;
}

/* Offer a burst: what the NIC's ring takes now; with
 * RUSTERON_DPDK_TX_RETRY_US set, the rest again as the NIC drains the ring,
 * until that long has passed. Returns the accepted count, a prefix of `pkts`;
 * the refused rest stays the caller's. */
static uint16_t rusteron_dpdk_port_tx_offer(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port, rusteron_dpdk_mbuf_t **pkts, uint16_t nb)
{
    const rusteron_dpdk_port_ops_t *ops = native->ops;
    uint16_t sent = ops->tx_burst(port->port_id, port->tx_queue_id, pkts, nb);
    if (sent < nb && native->tx_retry_ns > 0)
    {
        const uint64_t deadline = rusteron_dpdk_clock_now() + native->tx_retry_ns;
        do
        {
            sent += ops->tx_burst(port->port_id, port->tx_queue_id, pkts + sent, (uint16_t)(nb - sent));
        } while (sent < nb && rusteron_dpdk_clock_now() < deadline);
    }
    return sent;
}

/* Transmit a burst of control frames (ARP, ICMP): what the ring refuses is
 * released, as a dropped control frame is answered again. Returns the
 * accepted count, a prefix of `pkts`. */
uint16_t rusteron_dpdk_port_tx(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port, rusteron_dpdk_mbuf_t **pkts, uint16_t nb)
{
    const uint16_t sent = rusteron_dpdk_port_tx_offer(native, port, pkts, nb);
    for (uint16_t i = sent; i < nb; i++)
    {
        native->ops->mbuf_release(pkts[i]);
    }
    return sent;
}

int rusteron_dpdk_port_tx_flush(rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port)
{
    const uint16_t n = port->tx_queued_count;
    if (0 == n)
    {
        return 1;
    }
    rusteron_dpdk_mbuf_t *pkts[RUSTERON_DPDK_TX_VECTOR_MAX];
    for (uint16_t i = 0; i < n; i++)
    {
        pkts[i] = &port->tx_queued[i];
    }
    const uint16_t sent = native->ops->tx_burst(port->port_id, port->tx_queue_id, pkts, n);
    if (sent < n)
    {
        memmove(&port->tx_queued[0], &port->tx_queued[sent], (size_t)(n - sent) * sizeof(port->tx_queued[0]));
    }
    port->tx_queued_count = (uint16_t)(n - sent);
    return 0 == port->tx_queued_count;
}

void rusteron_dpdk_port_tx_drop_queued(rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port)
{
    for (uint16_t i = 0; i < port->tx_queued_count; i++)
    {
        native->ops->mbuf_release(&port->tx_queued[i]);
    }
    port->tx_queued_count = 0;
}

static int rusteron_dpdk_transport_send(
    aeron_udp_channel_data_paths_t *data_paths,
    aeron_udp_channel_transport_t *transport,
    struct sockaddr_storage *address,
    struct iovec *iov,
    size_t iov_length,
    int64_t *bytes_sent)
{
    (void)data_paths;

    rusteron_dpdk_client_t *client = transport->bindings_clientd;
    if (NULL == client || NULL == client->runtime || NULL == client->port)
    {
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport send: transport has no client state");
        rusteron_dpdk_set_error("DPDK transport send: transport has no client state");
        return -1;
    }
    if (NULL == bytes_sent || (NULL == iov && iov_length > 0))
    {
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport send: iov/bytes_sent must not be NULL");
        rusteron_dpdk_set_error("DPDK transport send: iov/bytes_sent must not be NULL");
        return -1;
    }
    if (0 == iov_length)
    {
        return 0;
    }

    if (client->delegated)
    {
        /* A host-local channel: its kernel socket, connected or not, as
         * Aeron's kernel transport sends. */
        int rc = rusteron_dpdk_kernel_send(
            client->kernel_fd, NULL != transport->connected_address ? NULL : address, iov, iov_length, bytes_sent);
        if (rc < 0)
        {
            rusteron_dpdk_set_error("DPDK transport send: host-local kernel send failed");
        }
        return rc;
    }

    /* The Aeron sender is a non-EAL thread; register it with DPDK before its
     * first tx (plan §7.2). The seam is a cheap lcore-id check after the first
     * call and is a no-op on already-registered/EAL threads. */
    if (rusteron_dpdk_eal_thread_register() < 0)
    {
        rusteron_dpdk_set_error("DPDK transport send: failed to register thread with DPDK");
        return -1;
    }

    rusteron_dpdk_transport_t *native = client->runtime;
    rusteron_dpdk_port_t *port = client->port;
    const rusteron_dpdk_port_ops_t *ops = native->ops;
    rusteron_dpdk_counters_t *counters = &port->counters;

    struct sockaddr_storage *dst = address != NULL ? address : transport->connected_address;
    if (NULL == dst)
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ERROR, 1);
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport send: no destination address");
        rusteron_dpdk_set_error("DPDK transport send: no destination address");
        return -1;
    }
    if (AF_INET != dst->ss_family)
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ERROR, 1);
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport send: only IPv4 unicast destinations are supported");
        rusteron_dpdk_set_error("DPDK transport send: only IPv4 unicast destinations are supported");
        return -1;
    }
    struct sockaddr_in *sin = (struct sockaddr_in *)dst;
    uint32_t dst_ip = sin->sin_addr.s_addr;
    uint16_t dst_port = (uint16_t)ntohs(sin->sin_port);

    if (rusteron_dpdk_ipv4_is_multicast(dst_ip))
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ERROR, 1);
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport send: multicast destinations are not supported");
        rusteron_dpdk_set_error("DPDK transport send: multicast destinations are not supported");
        return -1;
    }
    uint32_t net = rusteron_dpdk_ipv4_mask(port->prefix_len);
    if (INADDR_ANY == dst_ip || INADDR_BROADCAST == dst_ip || 0 == dst_port ||
        (31 > port->prefix_len && (dst_ip & net) == (port->local_ip & net) && (dst_ip & ~net) == ~net))
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ERROR, 1);
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport send: 0.0.0.0, broadcast or port 0 is not a destination");
        rusteron_dpdk_set_error("DPDK transport send: invalid destination");
        return -1;
    }

    /* A host-local destination (loopback, the host's kernel interfaces) is
     * unreachable through the NIC: it goes through a kernel socket bound to
     * this transport's own port, so the peer's replies come back to it. */
    if (rusteron_dpdk_is_host_local(native, dst_ip))
    {
        return rusteron_dpdk_transport_send_local(transport, client, dst, iov, iov_length, bytes_sent);
    }
    if (rusteron_dpdk_port_owns_ip(port, dst_ip))
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ERROR, 1);
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport send: the destination is this DPDK NIC's own address");
        rusteron_dpdk_set_error("DPDK transport send: destination is this NIC");
        return -1;
    }

    /* Route: an in-subnet next hop is ARP'd directly; anything else goes via
     * the configured gateway while keeping the destination IP in the header. */
    uint32_t next_hop = ((dst_ip & net) == (port->local_ip & net)) ? dst_ip : port->gateway_ip;

    const uint64_t now_ns = rusteron_dpdk_clock_now();
    uint8_t dst_mac[RUSTERON_DPDK_ETH_ADDR_LEN];
    const int resolved = rusteron_dpdk_arp_resolve(&port->arp, native, port, next_hop, now_ns, dst_mac);
    if (RUSTERON_DPDK_ARP_DROP == resolved)
    {
        /* Failed and already reported: drop, as an unconnected sendto to a
         * failed neighbour succeeds, so Aeron's other destinations carry on. */
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ARP_MISS, (int64_t)iov_length);
        for (size_t i = 0; i < iov_length; i++)
        {
            *bytes_sent += (int64_t)iov[i].iov_len;
        }
        return (int)iov_length;
    }
    if (RUSTERON_DPDK_ARP_FULL == resolved)
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ARP_MISS, 1);
        AERON_SET_ERR(ENOBUFS, "DPDK transport send: ARP table full (%d next hops in use)",
                      RUSTERON_DPDK_ARP_TABLE_SIZE);
        rusteron_dpdk_set_error("DPDK transport send: ARP table full");
        return -1;
    }
    if (resolved <= 0)
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ARP_MISS, 1);
        if (resolved < 0)
        {
            /* As Linux once a neighbour is NUD_FAILED: a hard error Aeron logs. */
            char ip_text[INET_ADDRSTRLEN] = "";
            inet_ntop(AF_INET, &next_hop, ip_text, sizeof(ip_text));
            AERON_SET_ERR(EHOSTUNREACH, "DPDK transport send: next hop %s does not answer ARP", ip_text);
            rusteron_dpdk_set_error("DPDK transport send: next hop unreachable (ARP failed)");
            return -1;
        }
        return 0; /* first resolution in flight; Aeron retries the zero result */
    }

    /* The source address, as the kernel would choose it: the bound one; for a
     * wildcard bind, the address this peer reached us on, else the NIC's. */
    uint32_t src_ip = client->bound_ip;
    if (0 == src_ip)
    {
        src_ip = port->local_ip;
        if (0 != port->extra_ip_count)
        {
            const rusteron_dpdk_reply_ip_t *r = &client->reply[rusteron_dpdk_reply_slot(dst_ip)];
            if (r->peer_ip == dst_ip && 0 != r->local_ip)
            {
                src_ip = r->local_ip;
            }
        }
    }

    /* Frames an earlier send queued behind a full ring go first. While any
     * still wait, this vector is refused whole before anything of it is
     * built or charged: Aeron offers it again, so a send is all or nothing
     * and no frame is ever sent twice. */
    if (0 != port->tx_queued_count && !rusteron_dpdk_port_tx_flush(native, port))
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_TX_EAGAIN, 1);
        return 0;
    }
    if (iov_length > RUSTERON_DPDK_TX_VECTOR_MAX)
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ERROR, 1);
        AERON_SET_ERR(EINVAL, "DPDK transport send: more than %d datagrams in one send", RUSTERON_DPDK_TX_VECTOR_MAX);
        rusteron_dpdk_set_error("DPDK transport send: too many datagrams in one send");
        return -1;
    }

    /* The bandwidth allowances: the NIC's (RUSTERON_DPDK_TX_RATE_MBPS, below
     * the VM's allowance) and the destination's route
     * (RUSTERON_DPDK_TX_ROUTE_MBPS, a slower link on the way), token buckets
     * owned by this NIC's agent, so plain arithmetic on the clock. A send may
     * overdraw them; while either is in debt the next send waits the few
     * microseconds the debt takes to repay, or, past
     * RUSTERON_DPDK_TX_PACE_SPIN_US, sends nothing (a zero result Aeron
     * retries whole: no partial send, so no duplicates;
     * RUSTERON_DPDK_TX_PACE_SPIN_US=0 always refuses instead of waiting).
     * Above a link's capacity the network drops packets unseen and Aeron
     * falls into retransmission; this keeps each below its own. Neither is
     * armed by default, so the whole block is skipped and no send pays for a
     * refill, a sum or a charge. Within it: no clock read, syscall, lock,
     * allocation or division of its own; a division by the rate and the
     * clock reads of a wait only in debt. */
    rusteron_dpdk_pacer_t *route = NULL;
    int charged = 0;
    if (0 != native->tx_route_count)
    {
        if (client->route_index == -2 || client->route_dst_ip != dst_ip)
        {
            client->route_dst_ip = dst_ip;
            client->route_index = -1;
            for (uint32_t r = 0; r < native->tx_route_count; r++)
            {
                if ((dst_ip & native->tx_route_mask[r]) == native->tx_route_net[r])
                {
                    client->route_index = (int)r; /* longest prefix first */
                    break;
                }
            }
        }
        if (client->route_index >= 0)
        {
            route = &port->route_pacers[client->route_index];
        }
    }
    if (0 != port->tx_pacer.rate_mbps || NULL != route)
    {
        uint64_t now = now_ns;
        rusteron_dpdk_pacer_refill(&port->tx_pacer, now);
        if (NULL != route)
        {
            rusteron_dpdk_pacer_refill(route, now);
        }
        int64_t wire = 0;
        for (size_t i = 0; i < iov_length; i++)
        {
            wire += (int64_t)iov[i].iov_len + RUSTERON_DPDK_ETH_HDR_LEN + 28; /* + IPv4 and UDP headers */
        }
        uint64_t wait_ns = 0;
        if (rusteron_dpdk_tx_low(native, iov, iov_length))
        {
            /* A stream that can wait (RUSTERON_DPDK_TX_LOW_STREAMS) sends only
             * from what leaves the reserve to the rest, and never waits: the
             * rest then never queue behind it. */
            if (!rusteron_dpdk_pacer_spare(&port->tx_pacer, wire, native->tx_low_reserve) ||
                (NULL != route && !rusteron_dpdk_pacer_spare(route, wire, native->tx_low_reserve)))
            {
                rusteron_dpdk_counters_add(counters, RD_COUNTER_TX_PACED, 1);
                return 0;
            }
        }
        else
        {
            wait_ns = rusteron_dpdk_pacer_wait(&port->tx_pacer);
            if (NULL != route && rusteron_dpdk_pacer_wait(route) > wait_ns)
            {
                wait_ns = rusteron_dpdk_pacer_wait(route);
            }
        }
        if (0 != wait_ns)
        {
            rusteron_dpdk_counters_add(counters, RD_COUNTER_TX_PACED, 1);
            /* Past the spin limit the send goes later, whole. */
            if (wait_ns > native->tx_pace_spin_ns)
            {
                return 0;
            }
            const uint64_t until = now + wait_ns;
            /* bounded: a clock pinned by a test never reaches `until` */
            for (uint32_t spins = 0; now < until && spins < (1U << 24); spins++)
            {
                rusteron_dpdk_spin_hint();
                now = rusteron_dpdk_clock_now();
            }
            rusteron_dpdk_pacer_refill(&port->tx_pacer, now);
            if (NULL != route)
            {
                rusteron_dpdk_pacer_refill(route, now);
            }
        }
        if (0 != port->tx_pacer.rate_mbps)
        {
            rusteron_dpdk_pacer_charge(&port->tx_pacer, wire);
        }
        if (NULL != route)
        {
            rusteron_dpdk_pacer_charge(route, wire);
        }
        charged = 1;
    }

    rusteron_dpdk_mbuf_t views[RUSTERON_DPDK_TX_VECTOR_MAX];
    rusteron_dpdk_mbuf_t *batch[RUSTERON_DPDK_TX_VECTOR_MAX];
    size_t built = 0;
    int oversized = 0;
    int nobufs = 0;

    /* Every iovec is one datagram (plan §7.4): build them all first, so the
     * vector is sent or queued whole. */
    for (size_t i = 0; i < iov_length; i++)
    {
        if (iov[i].iov_len > client->mtu)
        {
            oversized = 1;
            break;
        }
        rusteron_dpdk_mbuf_t *m = &views[built];
        if (ops->mbuf_alloc(port->mempool, m) < 0)
        {
            nobufs = 1;
            break;
        }
        if (rusteron_dpdk_packet_build_udp(
                m, dst_mac, port->mac, dst_ip, src_ip,
                client->local_udp_port, dst_port, &iov[i], 1, client->mtu,
                port->tx_udp_csum_offload) < 0)
        {
            ops->mbuf_release(m);
            oversized = 1;
            break;
        }
        batch[built++] = m;
    }
    if (nobufs)
    {
        /* Out of mbufs part-way: nothing of the vector goes, as part of it
         * would make Aeron send the whole vector again, that part twice. */
        rusteron_dpdk_counters_add(counters, RD_COUNTER_NOBUFS, 1);
        for (size_t i = 0; i < built; i++)
        {
            ops->mbuf_release(batch[i]);
        }
        built = 0;
    }

    /* To the ring a burst at a time; whatever it refuses is queued, built, to
     * go first at this agent's next send or poll. The queue is empty here. */
    size_t accepted = 0;
    while (accepted < built)
    {
        size_t chunk = built - accepted;
        if (chunk > native->config.burst_size)
        {
            chunk = native->config.burst_size;
        }
        const uint16_t bs = rusteron_dpdk_port_tx_offer(native, port, batch + accepted, (uint16_t)chunk);
        accepted += bs;
        if (bs < chunk)
        {
            break;
        }
    }
    if (accepted < built)
    {
        const size_t queued = built - accepted;
        for (size_t i = 0; i < queued; i++)
        {
            port->tx_queued[i] = *batch[accepted + i];
        }
        port->tx_queued_count = (uint16_t)queued;
        rusteron_dpdk_counters_add(counters, RD_COUNTER_TX_EAGAIN, 1);
        rusteron_dpdk_counters_add(counters, RD_COUNTER_TX_QUEUED, (int64_t)queued);
    }
    /* Sent or queued: either way Aeron's to forget. */
    const size_t sent = built;

    /* Frames charged to the allowances above but neither sent nor queued
     * (no mbufs, an oversized datagram and those after it): refund them, as
     * Aeron offers them again. */
    if (charged && sent < iov_length)
    {
        int64_t unsent = 0;
        for (size_t i = sent; i < iov_length; i++)
        {
            unsent += (int64_t)iov[i].iov_len + RUSTERON_DPDK_ETH_HDR_LEN + 28;
        }
        if (0 != port->tx_pacer.rate_mbps)
        {
            rusteron_dpdk_pacer_charge(&port->tx_pacer, -unsent);
        }
        if (NULL != route)
        {
            rusteron_dpdk_pacer_charge(route, -unsent);
        }
    }

    /* bytes_sent accrues for the datagrams sent or queued (plan §7.4). */
    for (size_t i = 0; i < sent; i++)
    {
        *bytes_sent += iov[i].iov_len;
    }
    rusteron_dpdk_counters_add(counters, RD_COUNTER_TX_PKTS, (int64_t)sent);
    rusteron_dpdk_counters_add(counters, RD_COUNTER_TX_BYTES, *bytes_sent);

    if (oversized)
    {
        rusteron_dpdk_counters_add(counters, RD_COUNTER_ERROR, 1);
        /* The prefix was sent and accounted; the oversized datagram itself is a
         * permanent validation error surfaced through the Aeron mechanism. */
        AERON_SET_ERR(EINVAL, "%s", "DPDK transport send: datagram exceeds the channel MTU");
        rusteron_dpdk_set_error("DPDK transport send: oversized datagram rejected");
        return -1;
    }

    return (int)sent;
}

static int rusteron_dpdk_transport_get_so_rcvbuf(
    aeron_udp_channel_transport_t *transport, size_t *so_rcvbuf)
{
    if (NULL == so_rcvbuf)
    {
        rusteron_dpdk_set_error("so_rcvbuf out-param must not be NULL");
        return -1;
    }
    *so_rcvbuf = 0;
    rusteron_dpdk_client_t *client = NULL != transport ? transport->bindings_clientd : NULL;
    if (NULL != client && client->delegated)
    {
        int value = 0;
        socklen_t len = sizeof(value);
        if (getsockopt(client->kernel_fd, SOL_SOCKET, SO_RCVBUF, &value, &len) < 0)
        {
            AERON_SET_ERR(errno, "%s", "DPDK transport: getsockopt(SO_RCVBUF)");
            return -1;
        }
        *so_rcvbuf = (size_t)value;
    }
    return 0;
}

/* "ip:port", the shape aeron_format_source_identity produces for IPv4. A DPDK
 * transport reports its NIC's address even for a wildcard bind (getsockname
 * would say 0.0.0.0): peers can use it. A host-local channel reports its
 * kernel socket's. */
static int rusteron_dpdk_transport_bind_addr_and_port(
    aeron_udp_channel_transport_t *transport, char *buffer, size_t length)
{
    if (NULL == buffer || length == 0)
    {
        rusteron_dpdk_set_error("bind_addr_and_port buffer must not be NULL/empty");
        return -1;
    }

    rusteron_dpdk_client_t *client = transport->bindings_clientd;
    if (NULL == client || NULL == client->port)
    {
        rusteron_dpdk_set_error("DPDK transport bind_addr_and_port: no client state");
        return -1;
    }

    if (client->delegated)
    {
        struct sockaddr_in bound;
        socklen_t len = sizeof(bound);
        char ip_text[INET_ADDRSTRLEN] = "";
        if (getsockname(client->kernel_fd, (struct sockaddr *)&bound, &len) < 0)
        {
            AERON_SET_ERR(errno, "%s", "DPDK transport: getsockname");
            return -1;
        }
        inet_ntop(AF_INET, &bound.sin_addr, ip_text, sizeof(ip_text));
        snprintf(buffer, length, "%s:%u", ip_text, (unsigned)ntohs(bound.sin_port));
        return 0;
    }
    char ip_text[INET_ADDRSTRLEN] = "";
    const uint32_t ip = 0 != client->bound_ip ? client->bound_ip : client->port->local_ip;
    inet_ntop(AF_INET, &ip, ip_text, sizeof(ip_text));
    snprintf(buffer, length, "%s:%u", ip_text, (unsigned)client->local_udp_port);
    return 0;
}

/* The poller callbacks live in rusteron_dpdk_poller.c (plan §7.6/§7.7). */
static int rusteron_dpdk_transport_poller_init(
    aeron_udp_transport_poller_t *poller,
    aeron_driver_context_t *context,
    aeron_udp_channel_transport_affinity_t affinity)
{
    return rusteron_dpdk_poller_init(poller, context, affinity);
}

static int rusteron_dpdk_transport_poller_close(aeron_udp_transport_poller_t *poller)
{
    return rusteron_dpdk_poller_close(poller);
}

static int rusteron_dpdk_transport_poller_add(
    aeron_udp_transport_poller_t *poller, aeron_udp_channel_transport_t *transport)
{
    return rusteron_dpdk_poller_add(poller, transport);
}

static int rusteron_dpdk_transport_poller_remove(
    aeron_udp_transport_poller_t *poller, aeron_udp_channel_transport_t *transport)
{
    return rusteron_dpdk_poller_remove(poller, transport);
}

static int rusteron_dpdk_transport_poller_poll(
    aeron_udp_transport_poller_t *poller,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    aeron_udp_channel_transport_recvmmsg_func_t recvmmsg_func,
    void *clientd)
{
    return rusteron_dpdk_poller_poll(
        poller, msgvec, vlen, bytes_rcved, recv_func, recvmmsg_func, clientd);
}

/* --- binding table --------------------------------------------------------- */

static aeron_udp_channel_transport_bindings_t rusteron_dpdk_transport_bindings_instance =
    {
        .init_func = rusteron_dpdk_transport_init,
        .reconnect_func = rusteron_dpdk_transport_reconnect,
        .close_func = rusteron_dpdk_transport_close_transport,
        .recvmmsg_func = rusteron_dpdk_transport_recvmmsg,
        .send_func = rusteron_dpdk_transport_send,
        .get_so_rcvbuf_func = rusteron_dpdk_transport_get_so_rcvbuf,
        .bind_addr_and_port_func = rusteron_dpdk_transport_bind_addr_and_port,
        .poller_init_func = rusteron_dpdk_transport_poller_init,
        .poller_close_func = rusteron_dpdk_transport_poller_close,
        .poller_add_func = rusteron_dpdk_transport_poller_add,
        .poller_remove_func = rusteron_dpdk_transport_poller_remove,
        .poller_poll_func = rusteron_dpdk_transport_poller_poll,
        .meta_info =
            {
                .name = RUSTERON_DPDK_BINDINGS_NAME,
                .type = RUSTERON_DPDK_BINDINGS_TYPE,
                .source_symbol = NULL,
            },
    };

aeron_udp_channel_transport_bindings_t *rusteron_dpdk_transport_bindings(void)
{
    rusteron_dpdk_transport_bindings_instance.meta_info.source_symbol = &rusteron_dpdk_transport_bindings_instance;
    return &rusteron_dpdk_transport_bindings_instance;
}

const char *rusteron_dpdk_last_error(void)
{
    if ('\0' == rusteron_dpdk_error_buffer[0])
    {
        return "no error";
    }

    return rusteron_dpdk_error_buffer;
}

int rusteron_dpdk_last_error_code(void)
{
    return rusteron_dpdk_error_code;
}
