/*
 * ergon-md-dpdk DPDK transport — runtime orchestration (plan §7.2).
 *
 * Owns the process-lifetime EAL singleton and drives both role ports through
 * the injectable port-ops seam: probe -> dev_info (net_ena + offloads) ->
 * dev_configure -> mempool -> rx/tx queue -> set_mtu -> watch resets -> start
 * -> link. Any
 * failure cleans up prior resources in reverse initialization order and the
 * error is recorded for rusteron_dpdk_last_error()/last_error_code().
 *
 * This translation unit references no libdpdk symbols directly: EAL goes
 * through the rusteron_dpdk_eal_* seam and ports through the port-ops table, so
 * it can be linked and deterministically tested without libdpdk.
 */

/* clock_gettime/CLOCK_MONOTONIC need POSIX 199309; the strict -std=c11 build
 * hides them unless this is declared before any libc header is included. */
#define _GNU_SOURCE /* pthread_setaffinity_np, cpu_set_t: the housekeeping thread */
#define _POSIX_C_SOURCE 200809L

#include "rusteron_dpdk_internal.h"

#include <arpa/inet.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sched.h>
#include <ifaddrs.h>
#include <netinet/in.h>
#include <sys/random.h>
#include <errno.h>
#include <strings.h>
#include <time.h>
#include <unistd.h>

#define RUSTERON_DPDK_IP_UDP_OVERHEAD 28 /* IPv4 + UDP headers above the payload */
#define RUSTERON_DPDK_LINK_TIMEOUT_MS 5000

/* Process-lifetime state. EAL cannot be reinitialized (plan §7.2), so the
 * ever-initialized flag is never cleared outside the test reset hook. */
static int rusteron_dpdk_test_eal_mode = RUSTERON_DPDK_EAL_REAL;
static int rusteron_dpdk_eal_ever_initialized = 0;
static const rusteron_dpdk_port_ops_t *rusteron_dpdk_port_ops_override = NULL;

void rusteron_dpdk_test_set_eal_mode(int mode)
{
    rusteron_dpdk_test_eal_mode = mode;
}

/* Transport clock: started at create (rusteron_dpdk_clock_start) from the
 * rate DPDK's EAL gives the counter, checked against a measurement of the
 * counter at start (rusteron_dpdk_clock_choose_rate). The hot path
 * (rusteron_dpdk_clock_now) reads the counter and converts it inline;
 * CLOCK_MONOTONIC and a test pin go through the slow path. Until a transport
 * starts it the clock is CLOCK_MONOTONIC with a factor of one: CLOCK_MONOTONIC
 * as it reads. */
#define RUSTERON_DPDK_CLOCK_BOOT \
    { .mode = RUSTERON_DPDK_CLOCK_MONOTONIC, .base_ticks = 0, .base_ns = 0, .mult = 1ULL << 32 }
rusteron_dpdk_clock_t rusteron_dpdk_clock = RUSTERON_DPDK_CLOCK_BOOT;
/* The started clock's mode, which a test pin hides until reset, and what the
 * start found (rusteron_dpdk_clock_report). */
static int rusteron_dpdk_clock_started_mode = RUSTERON_DPDK_CLOCK_MONOTONIC;
static rusteron_dpdk_clock_report_t rusteron_dpdk_clock_found = { .why = RUSTERON_DPDK_CLOCK_WHY_NOT_STARTED };
static uint64_t rusteron_dpdk_test_clock_ns_value = 0;
/* Reads of the pinned clock since the pin (tests: what a path costs). */
static uint64_t rusteron_dpdk_test_clock_read_count = 0;
/* The rate a test has the clock start take as measured (0: measure). */
static uint64_t rusteron_dpdk_test_measured_counter_hz = 0;
/* How far each of this thread's reads moves a pinned clock on: the test's
 * own, so the local-I/O thread's reads never move it under the test. */
static _Thread_local uint64_t rusteron_dpdk_test_clock_step_ns = 0;
/* The thread CPU clock, pinned by tests so a poll gap's split into work and
 * descheduling is deterministic (0: it is not pinned). */
static uint64_t rusteron_dpdk_test_thread_cpu_ns_value = 0;
static int rusteron_dpdk_test_thread_cpu_pinned = 0;
/* RUSTERON_DPDK_GAP_ATTRIB: RUSTERON_DPDK_GAP_ATTRIB_BURST times each PMD
 * burst; RUSTERON_DPDK_GAP_ATTRIB_CPU reads the thread CPU clock every receive
 * poll and splits each poll gap into work and descheduling. 1 sets both,
 * `burst` the first. Off by default, so the hot path never reads either.
 * Hidden and read inline (rusteron_dpdk_gap_attrib_on, _cpu_on). */
int rusteron_dpdk_gap_attrib = 0;

static uint64_t rusteron_dpdk_clock_read_ns(clockid_t id)
{
    struct timespec ts;
    clock_gettime(id, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

static uint64_t rusteron_dpdk_monotonic_ns(void)
{
    return rusteron_dpdk_clock_read_ns(CLOCK_MONOTONIC);
}

int rusteron_dpdk_clock_rule(int source, const rusteron_dpdk_counter_rate_t *rate)
{
    if (rate->eal_hz < RUSTERON_DPDK_CLOCK_MIN_HZ || rate->eal_hz > RUSTERON_DPDK_CLOCK_MAX_HZ)
    {
        return RUSTERON_DPDK_CLOCK_WHY_NO_RATE;
    }
    if (RUSTERON_DPDK_CLOCK_TSC == source && !rate->invariant)
    {
        return RUSTERON_DPDK_CLOCK_WHY_NOT_INVARIANT;
    }
    if (RUSTERON_DPDK_CLOCK_CNTVCT == source && rate->eal_hz != rate->cntfrq)
    {
        return RUSTERON_DPDK_CLOCK_WHY_NOT_CNTFRQ;
    }
    return RUSTERON_DPDK_CLOCK_WHY_COUNTER;
}

/* The counter and clock `id` at one instant: the tightest of `reads`
 * bracketed reads, the counter against the bracket's midpoint. */
static void rusteron_dpdk_clock_bracket(clockid_t id, int reads, uint64_t *ticks, uint64_t *ns)
{
    uint64_t best = UINT64_MAX;
    for (int i = 0; i < reads; i++)
    {
        const uint64_t before = rusteron_dpdk_clock_read_ns(id);
        const uint64_t t = rusteron_dpdk_counter();
        const uint64_t after = rusteron_dpdk_clock_read_ns(id);
        if (after - before < best)
        {
            best = after - before;
            *ticks = t;
            *ns = before + (after - before) / 2;
        }
    }
}

int rusteron_dpdk_clock_choose_rate(uint64_t eal_hz, uint64_t measured_hz, uint64_t *hz)
{
    *hz = eal_hz;
    if (measured_hz < RUSTERON_DPDK_CLOCK_MIN_HZ || measured_hz > RUSTERON_DPDK_CLOCK_MAX_HZ)
    {
        return RUSTERON_DPDK_CLOCK_RATE_EAL;
    }
    const uint64_t off = eal_hz > measured_hz ? eal_hz - measured_hz : measured_hz - eal_hz;
    /* off / measured <= tolerance / 1e6, in integers: both sides stay below
     * 2^64 over the range above. */
    if (off <= measured_hz && off * 1000000u <= measured_hz * RUSTERON_DPDK_CLOCK_RATE_TOLERANCE_PPM)
    {
        return RUSTERON_DPDK_CLOCK_RATE_EAL;
    }
    *hz = measured_hz;
    return RUSTERON_DPDK_CLOCK_RATE_MEASURED;
}

/* The counter's rate against CLOCK_MONOTONIC_RAW, which no NTP slew bends:
 * its ticks over RUSTERON_DPDK_CLOCK_MEASURE_NS, each end the tightest of 16
 * bracketed reads; 0 when no time passed. Spins for that long, so only the
 * clock start calls it, before the transport's threads and loop exist. */
static uint64_t rusteron_dpdk_clock_measure_hz(void)
{
    if (0 != rusteron_dpdk_test_measured_counter_hz)
    {
        return rusteron_dpdk_test_measured_counter_hz;
    }
    uint64_t t0 = 0, n0 = 0, t1 = 0, n1 = 0;
    rusteron_dpdk_clock_bracket(CLOCK_MONOTONIC_RAW, 16, &t0, &n0);
    while (rusteron_dpdk_clock_read_ns(CLOCK_MONOTONIC_RAW) - n0 < RUSTERON_DPDK_CLOCK_MEASURE_NS)
    {
        rusteron_dpdk_spin_hint();
    }
    rusteron_dpdk_clock_bracket(CLOCK_MONOTONIC_RAW, 16, &t1, &n1);
    const uint64_t ns = n1 - n0;
    if (0 == ns || n1 < n0)
    {
        return 0;
    }
    return (uint64_t)(((unsigned __int128)(t1 - t0) * 1000000000u + ns / 2) / ns);
}

void rusteron_dpdk_test_set_measured_counter_hz(uint64_t hz)
{
    rusteron_dpdk_test_measured_counter_hz = hz;
}

void rusteron_dpdk_clock_start(int choice)
{
    rusteron_dpdk_counter_rate_t rate = { 0 };
    rusteron_dpdk_eal_counter_rate(&rate);
    const int why = RUSTERON_DPDK_CLOCK_CHOICE_MONOTONIC == choice
        ? RUSTERON_DPDK_CLOCK_WHY_CHOSEN
        : rusteron_dpdk_clock_rule(RUSTERON_DPDK_CLOCK_COUNTER, &rate);
    rusteron_dpdk_clock_t clock = RUSTERON_DPDK_CLOCK_BOOT;
    uint64_t hz = 0;
    uint64_t measured_hz = 0;
    int rate_from = RUSTERON_DPDK_CLOCK_RATE_EAL;
    if (RUSTERON_DPDK_CLOCK_WHY_COUNTER == why)
    {
        measured_hz = rusteron_dpdk_clock_measure_hz();
        rate_from = rusteron_dpdk_clock_choose_rate(rate.eal_hz, measured_hz, &hz);
        /* ns per tick in 32.32, to the nearest: in (0, 2^63) over the rule's
         * range, which both rates choose_rate takes are in. Anchored on
         * CLOCK_MONOTONIC, so the time stays ns since boot. */
        clock.mult = (uint64_t)((((unsigned __int128)1000000000u << 32) + hz / 2) / hz);
        rusteron_dpdk_clock_bracket(CLOCK_MONOTONIC, 8, &clock.base_ticks, &clock.base_ns);
        clock.mode = RUSTERON_DPDK_CLOCK_COUNTER;
    }
    rusteron_dpdk_clock.base_ticks = clock.base_ticks;
    rusteron_dpdk_clock.base_ns = clock.base_ns;
    rusteron_dpdk_clock.mult = clock.mult;
    rusteron_dpdk_clock_found = (rusteron_dpdk_clock_report_t){
        .hz = hz,
        .eal_hz = rate.eal_hz,
        .cntfrq = rate.cntfrq,
        .measured_hz = measured_hz,
        .source = clock.mode,
        .why = why,
        .invariant = rate.invariant,
        .rate = rate_from,
    };
    rusteron_dpdk_clock_started_mode = clock.mode;
    if (RUSTERON_DPDK_CLOCK_PINNED != rusteron_dpdk_clock.mode)
    {
        rusteron_dpdk_clock.mode = clock.mode;
    }
}

void rusteron_dpdk_clock_report(rusteron_dpdk_clock_report_t *report)
{
    *report = rusteron_dpdk_clock_found;
}

uint64_t rusteron_dpdk_clock_ns_slow(void)
{
    if (RUSTERON_DPDK_CLOCK_PINNED == rusteron_dpdk_clock.mode)
    {
        __atomic_add_fetch(&rusteron_dpdk_test_clock_read_count, 1, __ATOMIC_RELAXED);
        const uint64_t now = rusteron_dpdk_test_clock_ns_value;
        if (0 != rusteron_dpdk_test_clock_step_ns)
        {
            rusteron_dpdk_test_clock_ns_value = now + rusteron_dpdk_test_clock_step_ns;
        }
        return now;
    }
    /* the fallback: CLOCK_MONOTONIC as its ticks */
    return rusteron_dpdk_clock_ns_at(rusteron_dpdk_monotonic_ns());
}

uint64_t rusteron_dpdk_clock_ns(void)
{
    return rusteron_dpdk_clock_now();
}

int rusteron_dpdk_clock_source(void)
{
    return rusteron_dpdk_clock_started_mode;
}

uint64_t rusteron_dpdk_test_clock_ns_at(uint64_t ticks)
{
    return rusteron_dpdk_clock_ns_at(ticks);
}

void rusteron_dpdk_test_clock_params(uint64_t *base_ticks, uint64_t *base_ns, uint64_t *mult)
{
    *base_ticks = rusteron_dpdk_clock.base_ticks;
    *base_ns = rusteron_dpdk_clock.base_ns;
    *mult = rusteron_dpdk_clock.mult;
}

uint64_t rusteron_dpdk_test_counter(void)
{
    return rusteron_dpdk_counter();
}

/* Move the transport's allowances onto a newly pinned clock: a pin is a
 * timeline of its own, and an allowance last refilled on the real clock
 * would credit nothing until the pin passed that time. */
static void rusteron_dpdk_test_pacers_onto(uint64_t ns)
{
    rusteron_dpdk_transport_t *native = rusteron_dpdk_active_runtime();
    if (NULL == native)
    {
        return;
    }
    rusteron_dpdk_port_t *ports[2] = { &native->sender, &native->receiver };
    for (int p = 0; p < 2; p++)
    {
        ports[p]->tx_pacer.last_ns = ns;
        for (uint32_t r = 0; r < RUSTERON_DPDK_MAX_TX_ROUTES; r++)
        {
            ports[p]->route_pacers[r].last_ns = ns;
        }
    }
}

void rusteron_dpdk_test_set_clock_ns(uint64_t ns)
{
    if (RUSTERON_DPDK_CLOCK_PINNED != rusteron_dpdk_clock.mode)
    {
        rusteron_dpdk_test_pacers_onto(ns);
    }
    rusteron_dpdk_test_clock_ns_value = ns;
    rusteron_dpdk_clock.mode = RUSTERON_DPDK_CLOCK_PINNED;
}

void rusteron_dpdk_test_set_clock_step_ns(uint64_t ns)
{
    rusteron_dpdk_test_clock_step_ns = ns;
}

void rusteron_dpdk_test_advance_clock_ns(uint64_t ns)
{
    if (RUSTERON_DPDK_CLOCK_PINNED == rusteron_dpdk_clock.mode)
    {
        rusteron_dpdk_test_clock_ns_value += ns;
    }
}

uint64_t rusteron_dpdk_test_clock_reads(void)
{
    return __atomic_load_n(&rusteron_dpdk_test_clock_read_count, __ATOMIC_RELAXED);
}

uint64_t rusteron_dpdk_thread_cpu_ns(void)
{
    if (rusteron_dpdk_test_thread_cpu_pinned)
    {
        return rusteron_dpdk_test_thread_cpu_ns_value;
    }
    struct timespec ts;
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

void rusteron_dpdk_test_set_thread_cpu_ns(uint64_t ns)
{
    rusteron_dpdk_test_thread_cpu_ns_value = ns;
    rusteron_dpdk_test_thread_cpu_pinned = 1;
}

int rusteron_dpdk_gap_attributing(void)
{
    return rusteron_dpdk_gap_attrib;
}

/* Whether runtime init starts the housekeeping thread: production yes; the
 * test reset turns it off so tests drive housekeeping with the tick hook. */
static int rusteron_dpdk_housekeeping_enabled = 1;

void rusteron_dpdk_test_set_housekeeping_thread(int enabled)
{
    rusteron_dpdk_housekeeping_enabled = enabled;
}

static uint64_t rusteron_dpdk_test_realtime_ns_value = 0;
static uint64_t rusteron_dpdk_test_realtime_step_ns = 0;
static int rusteron_dpdk_test_realtime_pinned = 0;
static uint64_t rusteron_dpdk_test_realtime_read_count = 0;

uint64_t rusteron_dpdk_realtime_ns(void)
{
    if (rusteron_dpdk_test_realtime_pinned)
    {
        rusteron_dpdk_test_realtime_read_count++;
        const uint64_t now = rusteron_dpdk_test_realtime_ns_value;
        rusteron_dpdk_test_realtime_ns_value += rusteron_dpdk_test_realtime_step_ns;
        return now;
    }
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

void rusteron_dpdk_test_set_realtime_ns(uint64_t ns)
{
    rusteron_dpdk_test_realtime_ns_value = ns;
    rusteron_dpdk_test_realtime_pinned = 1;
}

void rusteron_dpdk_test_set_realtime_step_ns(uint64_t ns)
{
    rusteron_dpdk_test_realtime_step_ns = ns;
}

uint64_t rusteron_dpdk_test_realtime_reads(void)
{
    return rusteron_dpdk_test_realtime_read_count;
}

/* A test's stand-in for every port's sysfs driver link, until reset. */
static char rusteron_dpdk_test_pci_kernel_driver_value[64] = "";

void rusteron_dpdk_test_set_pci_kernel_driver(const char *driver)
{
    snprintf(rusteron_dpdk_test_pci_kernel_driver_value, sizeof(rusteron_dpdk_test_pci_kernel_driver_value), "%s",
             NULL != driver ? driver : "");
}

/* The local-I/O mode a created runtime selected (RUSTERON_DPDK_LOCAL_IO_*,
 * -1 for no runtime). Every explicit RUSTERON_DPDK_LOCAL_IO value selects its
 * own mode and an unset one selects ADAPTIVE; reading it back is what proves
 * the selection, because an accepted environment value alone does not. */
int rusteron_dpdk_test_local_io_mode(const rusteron_dpdk_transport_t *native)
{
    return NULL != native ? native->local_io_mode : -1;
}

/* Clear a test pin and return the clock to its boot default, as a fresh
 * process has it: CLOCK_MONOTONIC with a factor of one, never started. */
static void rusteron_dpdk_test_clock_reset(void)
{
    const rusteron_dpdk_clock_t boot = RUSTERON_DPDK_CLOCK_BOOT;
    rusteron_dpdk_clock.mode = boot.mode;
    rusteron_dpdk_clock.base_ticks = boot.base_ticks;
    rusteron_dpdk_clock.base_ns = boot.base_ns;
    rusteron_dpdk_clock.mult = boot.mult;
    rusteron_dpdk_clock_started_mode = RUSTERON_DPDK_CLOCK_MONOTONIC;
    rusteron_dpdk_clock_found = (rusteron_dpdk_clock_report_t){ .why = RUSTERON_DPDK_CLOCK_WHY_NOT_STARTED };
    rusteron_dpdk_test_clock_ns_value = 0;
    rusteron_dpdk_test_clock_read_count = 0;
    rusteron_dpdk_test_measured_counter_hz = 0;
    rusteron_dpdk_test_clock_step_ns = 0;
    rusteron_dpdk_test_realtime_ns_value = 0;
    rusteron_dpdk_test_realtime_step_ns = 0;
    rusteron_dpdk_test_realtime_pinned = 0;
    rusteron_dpdk_test_realtime_read_count = 0;
}

void rusteron_dpdk_test_reset(void)
{
    rusteron_dpdk_test_pci_kernel_driver_value[0] = '\0';
    rusteron_dpdk_test_eal_mode = RUSTERON_DPDK_EAL_REAL;
    rusteron_dpdk_eal_ever_initialized = 0;
    rusteron_dpdk_port_ops_override = NULL;
    rusteron_dpdk_test_clock_reset();
    rusteron_dpdk_test_thread_cpu_pinned = 0;
    rusteron_dpdk_test_thread_cpu_ns_value = 0;
    rusteron_dpdk_gap_attrib = 0;
    rusteron_dpdk_events_test_reset();
    rusteron_dpdk_reports_test_reset();
    rusteron_dpdk_housekeeping_enabled = 0;
    rusteron_dpdk_transport_test_reset();
    /* Clear the thread-local error state so last_error()/last_error_code()
     * start from "no error" in the next test (tests run single-threaded). */
    rusteron_dpdk_set_error_code("", RUSTERON_DPDK_ERR_OK);
}

void rusteron_dpdk_set_port_ops(const rusteron_dpdk_port_ops_t *ops)
{
    rusteron_dpdk_port_ops_override = ops;
}

static const rusteron_dpdk_port_ops_t *rusteron_dpdk_active_ops(void)
{
    if (NULL != rusteron_dpdk_port_ops_override)
    {
        return rusteron_dpdk_port_ops_override;
    }
    return rusteron_dpdk_port_ops_real();
}

/* Parse a dotted-quad into a network-order uint32_t; 0 on success. */
static int rusteron_dpdk_parse_ipv4(const char *text, uint32_t *out)
{
    struct in_addr addr;
    if (NULL == text || 1 != inet_pton(AF_INET, text, &addr))
    {
        return -1;
    }
    *out = addr.s_addr;
    return 0;
}

/* Bring EAL up once per process (or skip it under the test hook). The EAL
 * singleton guard fires in every mode, including the test SKIP mode, so a
 * second transport in one process always reports ALREADY_INITIALIZED. */
static int rusteron_dpdk_runtime_eal_up(rusteron_dpdk_transport_t *native)
{
    if (rusteron_dpdk_eal_ever_initialized || rusteron_dpdk_eal_is_initialized())
    {
        rusteron_dpdk_set_error_code(
            "DPDK EAL is already initialized in this process (only one EAL runtime is permitted)",
            RUSTERON_DPDK_ERR_ALREADY_INITIALIZED);
        return -1;
    }

    /* RUSTERON_DPDK_EAL_IN_MEMORY and RUSTERON_DPDK_EAL_ARGS agree, or
     * nothing starts. */
    int in_memory = 0;
    if (rusteron_dpdk_eal_in_memory(&in_memory) < 0)
    {
        return -1;
    }
    if (RUSTERON_DPDK_EAL_SKIP == rusteron_dpdk_test_eal_mode)
    {
        native->eal_up = 1;
        rusteron_dpdk_eal_ever_initialized = 1;
        return 0;
    }

    rusteron_dpdk_eal_params_t params;
    memset(&params, 0, sizeof(params));
    params.config = &native->config;
    params.mode = rusteron_dpdk_test_eal_mode;

    char errbuf[512] = "";
    if (rusteron_dpdk_eal_init(&params, errbuf, sizeof(errbuf)) < 0)
    {
        rusteron_dpdk_set_error(errbuf[0] != '\0' ? errbuf : "DPDK EAL initialization failed");
        return -1;
    }

    native->eal_up = 1;
    rusteron_dpdk_eal_ever_initialized = 1;
    return 0;
}

/* The kernel driver `bdf` is bound to (its sysfs driver link), or "-". */
static void rusteron_dpdk_pci_kernel_driver(const char *bdf, char *out, size_t len)
{
    if ('\0' != rusteron_dpdk_test_pci_kernel_driver_value[0])
    {
        snprintf(out, len, "%s", rusteron_dpdk_test_pci_kernel_driver_value);
        return;
    }
    char path[96], target[256];
    snprintf(path, sizeof(path), "/sys/bus/pci/devices/%s/driver", bdf);
    const ssize_t n = readlink(path, target, sizeof(target) - 1);
    if (n <= 0)
    {
        snprintf(out, len, "-");
        return;
    }
    target[n] = '\0';
    const char *slash = strrchr(target, '/');
    snprintf(out, len, "%s", NULL != slash ? slash + 1 : target);
}

/* An ENA port's low-latency queue (LLQ): without it, or with its BAR mapped
 * uncached, every send costs about 100 times as much (Aeron Premium's DPDK
 * notes; DPDK 24.11.0 and .1 turned it off by default). The transport names
 * the LLQ policy in the port's device arguments (rusteron_dpdk_ena_devargs);
 * no DPDK API or xstat reports the placement the PMD chose, so what is checked
 * is that the operator did not turn it off and that the process maps the
 * port's LLQ BAR write-combined (dev_info's ena_llq, from /proc/self/maps, as
 * benches/cloud/host.sh's ena_llq_check). vfio-pci maps the BAR through its
 * device file, which the map shows as neither, so a port bound to vfio-pci
 * whose mapping is unseen starts on RUSTERON_DPDK_VFIO_WC=1: the operator's
 * confirmation of a write-combining vfio-pci, which deploy/aws-eks's preflight
 * requires before the bootstrap binds one. Every other port is checked as
 * before. Start fails naming the port unless RUSTERON_DPDK_ENA_LLQ_REQUIRED=0,
 * which starts it with a warning. */
static int rusteron_dpdk_ena_llq_check(const rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *p)
{
    char devargs[RUSTERON_DPDK_ENA_DEVARGS_MAX + 32];
    int llq_off = 0;
    if (rusteron_dpdk_ena_devargs(devargs, sizeof(devargs), &llq_off) < 0)
    {
        return -1;
    }
    if (llq_off)
    {
        p->ena_llq = RUSTERON_DPDK_ENA_LLQ_DISABLED;
    }
    char kdrv[64];
    rusteron_dpdk_pci_kernel_driver(p->pci, kdrv, sizeof(kdrv));
    char why[512];
    switch (p->ena_llq)
    {
        case RUSTERON_DPDK_ENA_LLQ_WC:
            return 0;
        case RUSTERON_DPDK_ENA_LLQ_DISABLED:
            snprintf(why, sizeof(why), "RUSTERON_DPDK_ENA_DEVARGS turns its low-latency queue (LLQ) off (%s)", devargs);
            break;
        case RUSTERON_DPDK_ENA_LLQ_UNCACHED:
            snprintf(why, sizeof(why),
                     "its low-latency queue (LLQ) BAR, BAR 2, is mapped uncached (resource2, not resource2_wc; bound "
                     "to %s): load igb_uio with wc_activate=1", kdrv);
            break;
        case RUSTERON_DPDK_ENA_LLQ_UNSEEN:
            if (native->vfio_wc && 0 == strcmp(kdrv, "vfio-pci"))
            {
                return 0;
            }
            snprintf(why, sizeof(why),
                     "no write-combined mapping of its low-latency queue (LLQ) BAR (BAR 2, resource2_wc) in "
                     "/proc/self/maps (bound to %s): vfio-pci maps it through its device file, which shows neither; "
                     "on a write-combining vfio-pci (Amazon's patch) set RUSTERON_DPDK_VFIO_WC=1, as "
                     "deploy/aws-eks's preflight requires, else bind it to igb_uio (wc_activate=1)", kdrv);
            break;
        default:
            snprintf(why, sizeof(why),
                     "/proc/self/maps cannot be read, so its low-latency queue (LLQ) BAR's write-combined mapping "
                     "cannot be confirmed");
            break;
    }
    char message[768];
    snprintf(message, sizeof(message),
             "ENA %s (port %u): %s. Without LLQ a send costs about 100 times as much; "
             "RUSTERON_DPDK_ENA_LLQ_REQUIRED=0 starts it anyway, for diagnosis",
             p->pci, (unsigned)p->port_id, why);
    if (native->ena_llq_required)
    {
        rusteron_dpdk_set_error(message);
        return -1;
    }
    fprintf(stderr, "WARNING ergon-md-dpdk: %s: starting anyway (RUSTERON_DPDK_ENA_LLQ_REQUIRED=0)\n", message);
    return 0;
}

/* ENA Express (SRD) carries UDP only in IP packets of at most this many
 * bytes (AWS: ENA Express's UDP mode needs an MTU of 8900 or less); the
 * ena_srd_mode xstat's bit 1 says its UDP mode is on (0 off, 1 SRD for TCP
 * only, 2 or 3 UDP too). A larger packet is not lost: measured on
 * c6gn.4xlarge with kernel UDP (ENA driver 2.17.4g, 2026-10-25), UDP of
 * 8,901 to 9,001 IP bytes arrived whole, counted in ena_srd_eligible_tx_pkts
 * but not in ena_srd_tx_pkts: ENA sent it without SRD. */
#define RUSTERON_DPDK_ENA_EXPRESS_UDP_MTU 8900
#define RUSTERON_DPDK_ENA_SRD_MODE_UDP 0x2ULL

/* Warn, once at start, when an ENA runs ENA Express's UDP over SRD and this
 * transport's IP packets (l3_mtu) can exceed what SRD carries: those leave
 * SRD for ENA's plain path (its bandwidth and tail-latency gains do not
 * apply to them), while smaller frames of the same flow stay on SRD. Read
 * from the PMD's ena_srd_mode xstat (on ENA an admin command); a port
 * without it, or with an MTU within the limit, says nothing. Never refuses:
 * nothing is lost. */
static int rusteron_dpdk_ena_express_check(const rusteron_dpdk_transport_t *native, const rusteron_dpdk_port_t *p,
                                           uint16_t l3_mtu)
{
    const rusteron_dpdk_port_ops_t *ops = native->ops;
    if (l3_mtu <= RUSTERON_DPDK_ENA_EXPRESS_UDP_MTU || NULL == ops->xstats_count || NULL == ops->xstats_names ||
        NULL == ops->xstats_get)
    {
        return 0;
    }
    const uint32_t n = ops->xstats_count(p->port_id);
    if (0 == n || n > RUSTERON_DPDK_MAX_XSTATS)
    {
        return 0;
    }
    char *names = calloc(n, RUSTERON_DPDK_XSTAT_NAME_LEN);
    if (NULL == names)
    {
        return 0;
    }
    uint32_t srd = UINT32_MAX;
    if (0 == ops->xstats_names(p->port_id, names, n))
    {
        for (uint32_t i = 0; i < n && UINT32_MAX == srd; i++)
        {
            if (0 == strncmp(names + (size_t)i * RUSTERON_DPDK_XSTAT_NAME_LEN, "ena_srd_mode",
                             RUSTERON_DPDK_XSTAT_NAME_LEN))
            {
                srd = i;
            }
        }
    }
    free(names);
    uint64_t values[RUSTERON_DPDK_MAX_XSTATS];
    if (UINT32_MAX == srd || 0 != ops->xstats_get(p->port_id, values, n) ||
        0 == (values[srd] & RUSTERON_DPDK_ENA_SRD_MODE_UDP))
    {
        return 0;
    }
    fprintf(stderr,
            "WARNING ergon-md-dpdk: %s (port %u) runs ENA Express with UDP over SRD (ena_srd_mode %llu), which "
            "carries IP packets of at most %d bytes; max_aeron_mtu %zu makes packets of up to %u, and ENA sends "
            "the larger ones without SRD. RUSTERON_DPDK_MAX_AERON_MTU of at most %d keeps every frame on SRD\n",
            p->pci, (unsigned)p->port_id, (unsigned long long)values[srd], RUSTERON_DPDK_ENA_EXPRESS_UDP_MTU,
            native->config.max_aeron_mtu, (unsigned)l3_mtu,
            (RUSTERON_DPDK_ENA_EXPRESS_UDP_MTU - RUSTERON_DPDK_IP_UDP_OVERHEAD) / 32 * 32);
    return 0;
}

/* Azure: a netvsc port moves data through its accelerated-networking VF only
 * while one is attached; without it every packet takes the synthetic VMBus
 * channel through the hypervisor (measured: millions of HYP interrupts a
 * point, slower than kernel UDP), and nothing else fails. ethdev has no call
 * that says whether a VF is attached; what shows it is the port's extended
 * statistics: netvsc appends its VF's, each prefixed "vf_"
 * (hn_vf_xstats_get_names, DPDK 23.11 to 25.11), and reports none without
 * one, its own being "rx_q<n>_*" and "tx_q<n>_*". netvsc attaches a VF it
 * cannot yet find a second later (hn_vf_add's retry alarm), so start waits up
 * to this long after link up. */
#define RUSTERON_DPDK_NETVSC_VF_WAIT_MS 3000
#define RUSTERON_DPDK_NETVSC_VF_POLL_MS 100

/* Whether the port reports any "vf_" extended statistic. */
static int rusteron_dpdk_netvsc_vf_attached(const rusteron_dpdk_port_ops_t *ops, uint16_t port_id)
{
    if (NULL == ops->xstats_count || NULL == ops->xstats_names)
    {
        return 0;
    }
    const uint32_t n = ops->xstats_count(port_id);
    if (0 == n)
    {
        return 0;
    }
    char *names = calloc(n, RUSTERON_DPDK_XSTAT_NAME_LEN);
    if (NULL == names)
    {
        return 0;
    }
    int found = 0;
    if (0 == ops->xstats_names(port_id, names, n))
    {
        for (uint32_t i = 0; i < n && !found; i++)
        {
            found = 0 == strncmp(names + (size_t)i * RUSTERON_DPDK_XSTAT_NAME_LEN, "vf_", 3);
        }
    }
    free(names);
    return found;
}

/* Fail start for a netvsc port with no VF attached (RUSTERON_DPDK_ERR_NO_ACCELERATED_VF),
 * or, under RUSTERON_DPDK_ALLOW_SYNTHETIC=1, warn and go on. */
static int rusteron_dpdk_netvsc_vf_check(const rusteron_dpdk_transport_t *native, const rusteron_dpdk_port_t *p)
{
    const struct timespec step = { 0, RUSTERON_DPDK_NETVSC_VF_POLL_MS * 1000000L };
    for (uint32_t waited = 0;; waited += RUSTERON_DPDK_NETVSC_VF_POLL_MS)
    {
        if (rusteron_dpdk_netvsc_vf_attached(native->ops, p->port_id))
        {
            return 0;
        }
        if (waited >= RUSTERON_DPDK_NETVSC_VF_WAIT_MS)
        {
            break;
        }
        nanosleep(&step, NULL);
    }
    char message[1024];
    snprintf(message, sizeof(message),
             "net_netvsc port %u on %s (MAC %s) has no accelerated-networking VF attached (no vf_* extended "
             "statistics %d ms after link up): it would run on the synthetic VMBus path, every packet through the "
             "hypervisor, slower than kernel UDP. This set-up supports MANA VFs only; a Mellanox (mlx5) VF is not "
             "supported. Use a VM size with MANA and accelerated networking on, with the MANA PCI function probed "
             "by DPDK (mana_ib loaded, the VF's kernel interface down). RUSTERON_DPDK_ALLOW_SYNTHETIC=1 starts it "
             "anyway, for diagnosis only",
             (unsigned)p->port_id, p->pci, p->mac_text, RUSTERON_DPDK_NETVSC_VF_WAIT_MS);
    if (!native->allow_synthetic)
    {
        rusteron_dpdk_set_error_code(message, RUSTERON_DPDK_ERR_NO_ACCELERATED_VF);
        return -1;
    }
    fprintf(stderr,
            "WARNING ergon-md-dpdk: %s. STARTING ANYWAY ON THE SYNTHETIC PATH (RUSTERON_DPDK_ALLOW_SYNTHETIC=1): "
            "no measurement of this port is kernel bypass\n",
            message);
    return 0;
}

static int rusteron_dpdk_init_port(rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *p)
{
    const rusteron_dpdk_port_ops_t *ops = native->ops;
    const rusteron_dpdk_config_t *cfg = &native->config;
    char message[512];

    if (ops->probe_port(p->pci, p->has_mac ? p->want_mac : NULL, &p->port_id) < 0)
    {
        if (p->has_mac)
        {
            snprintf(message, sizeof(message),
                     "no DPDK port on %s reports MAC %s (is the MANA interface down and the MAC its own?)",
                     p->pci, p->mac_text);
        }
        else
        {
            snprintf(message, sizeof(message),
                     "device %s is not probed by DPDK (allow-listed ENA must be bound via VFIO or igb_uio, and its "
                     "PMD must accept its device arguments: the LLQ policy and RUSTERON_DPDK_ENA_DEVARGS)", p->pci);
        }
        rusteron_dpdk_set_error(message);
        return -1;
    }

    /* One NUMA node for both ENAs. A node the PMD does not report (-1) does
     * not contradict the other port. Queues and the pool use the reported
     * node; a hardcoded 0 would place them on the wrong node when the only
     * node is not 0. */
    p->socket_id = NULL != ops->socket_id ? ops->socket_id(p->port_id) : -1;
    p->socket_known = 1;
    {
        const rusteron_dpdk_port_t *peer = (p == &native->sender) ? &native->receiver : &native->sender;
        if (p->socket_id >= 0 && peer->socket_known && peer->socket_id >= 0 && peer->socket_id != p->socket_id)
        {
            snprintf(message, sizeof(message),
                     "sender %s is on NUMA node %d and receiver %s is on NUMA node %d; both ENAs must be on one "
                     "NUMA node (this driver does not place queues across nodes)",
                     native->sender.pci, native->sender.socket_id, native->receiver.pci, p->socket_id);
            rusteron_dpdk_set_error(message);
            return -1;
        }
    }

    char driver[64] = "";
    uint16_t desc_lim[4] = { 0, 0, 0, 0 };
    if (ops->dev_info(p->port_id, p->pci, p->mac, driver, sizeof(driver),
                      &p->csum_offload_ok, &p->ena_llq, &p->rx_timestamp_ok, &p->tx_fast_free_ok, &p->max_mtu,
                      desc_lim) < 0)
    {
        snprintf(message, sizeof(message),
                 "cannot read capabilities of device %s (port %u)", p->pci, p->port_id);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    if (0 == memcmp(p->mac, "\x00\x00\x00\x00\x00\x00", 6))
    {
        snprintf(message, sizeof(message),
                 "device %s (port %u) reports an all-zero MAC", p->pci, p->port_id);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    /* Keep the PMD name for the port-info counter label (plan §9). */
    memcpy(p->driver, driver, sizeof(p->driver));
    const int pci = rusteron_dpdk_selector_is_pci(p->pci);
    const int ena = pci && 0 == strcmp(driver, "net_ena");
    /* Azure: net_netvsc (the recommended path) drives the NIC's synthetic
     * channel and the MANA VF beneath it; net_mana is the VF alone. */
    const int azure = pci && (0 == strcmp(driver, "net_mana") || 0 == strcmp(driver, "net_netvsc"));
    /* The receive ring: as configured, else RUSTERON_DPDK_RX_DESCRIPTORS_DEFAULT
     * or the NIC's maximum where lower: MANA's 256, which netvsc sets its VF
     * up with whatever it reports itself. A power of two, as ENA's and MANA's
     * rings must be. 4096 against ENA's 1024 did not separate at idle
     * (md-dpdk/BENCHMARKS.md, "Measured, but no setting recommended"). */
    p->rx_descriptors = cfg->rx_descriptors;
    if (0 == p->rx_descriptors)
    {
        unsigned rx = RUSTERON_DPDK_RX_DESCRIPTORS_DEFAULT;
        if (azure && rx > RUSTERON_DPDK_MANA_MAX_DESCRIPTORS)
        {
            rx = RUSTERON_DPDK_MANA_MAX_DESCRIPTORS;
        }
        if (0 != desc_lim[1] && desc_lim[1] < rx)
        {
            rx = desc_lim[1];
        }
        while (0 != (rx & (rx - 1)))
        {
            rx &= rx - 1;
        }
        p->rx_descriptors = (uint16_t)rx;
    }
    /* Azure's MANA overflows its ring without counting a drop, so a poll that
     * drains within a burst of a full ring is suspected of it; an ENA counts
     * the drops itself (imissed: nic-rx-missed), and has no suspicion. */
    p->rx_ring_full_at = ena ? UINT32_MAX
        : p->rx_descriptors > cfg->burst_size ? (uint32_t)p->rx_descriptors - cfg->burst_size : 0;
    /* The PCI-device guarantees (a supported PMD, IPv4/UDP checksum offload)
     * are required for a PCI NIC but not for a virtual/TAP device: vdev PMDs
     * report their own driver name and may software-checksum (plan §11.2). */
    if (pci)
    {
        if (!ena && !azure)
        {
            snprintf(message, sizeof(message),
                     "device %s reports driver %s, expected the net_ena (AWS) or net_mana / net_netvsc (Azure) PMD",
                     p->pci, driver[0] != '\0' ? driver : "<unknown>");
            rusteron_dpdk_set_error(message);
            return -1;
        }
        if (azure && !p->has_mac)
        {
            /* Without a MAC the MANA PMD takes every vport on the function,
             * the primary NIC included. */
            snprintf(message, sizeof(message),
                     "device %s is a %s port: set the port's MAC "
                     "(RUSTERON_DPDK_SENDER_MAC / RUSTERON_DPDK_RECEIVER_MAC)", p->pci, driver);
            rusteron_dpdk_set_error(message);
            return -1;
        }
        if (azure && (p->rx_descriptors > RUSTERON_DPDK_MANA_MAX_DESCRIPTORS ||
                      cfg->tx_descriptors > RUSTERON_DPDK_MANA_MAX_DESCRIPTORS))
        {
            /* netvsc sets up its MANA VF's queues with the same sizes. */
            snprintf(message, sizeof(message),
                     "%s queues (a MANA VF) hold at most %d descriptors (rx %u, tx %u): set "
                     "RUSTERON_DPDK_RX_DESCRIPTORS and RUSTERON_DPDK_TX_DESCRIPTORS to %d",
                     driver, RUSTERON_DPDK_MANA_MAX_DESCRIPTORS, p->rx_descriptors, cfg->tx_descriptors,
                     RUSTERON_DPDK_MANA_MAX_DESCRIPTORS);
            rusteron_dpdk_set_error(message);
            return -1;
        }
        if (p->has_mac && 0 != memcmp(p->mac, p->want_mac, RUSTERON_DPDK_ETH_ADDR_LEN))
        {
            snprintf(message, sizeof(message),
                     "port %u on %s does not report the configured MAC %s", p->port_id, p->pci, p->mac_text);
            rusteron_dpdk_set_error(message);
            return -1;
        }
        if (!p->csum_offload_ok)
        {
            snprintf(message, sizeof(message),
                     "device %s lacks the required IPv4/UDP checksum offloads", p->pci);
            rusteron_dpdk_set_error(message);
            return -1;
        }
        if (ena && rusteron_dpdk_ena_llq_check(native, p) < 0)
        {
            return -1;
        }
    }

    if (cfg->rx_hw_timestamp && !p->rx_timestamp_ok)
    {
        snprintf(message, sizeof(message),
                 "device %s (driver %s) does not offer hardware receive timestamps "
                 "(RX_OFFLOAD_TIMESTAMP: AWS ENA in a precision-time placement group, DPDK >= 25.11); "
                 "unset RUSTERON_DPDK_RX_HW_TIMESTAMP",
                 p->pci, driver[0] != '\0' ? driver : "<unknown>");
        rusteron_dpdk_set_error(message);
        return -1;
    }

    uint16_t l3_mtu = (uint16_t)(cfg->max_aeron_mtu + RUSTERON_DPDK_IP_UDP_OVERHEAD);
    if (0 != p->max_mtu && l3_mtu > p->max_mtu)
    {
        snprintf(message, sizeof(message),
                 "max_aeron_mtu %zu needs an L3 MTU of %u, above %s's maximum %u (driver %s): lower "
                 "RUSTERON_DPDK_MAX_AERON_MTU to at most %u",
                 cfg->max_aeron_mtu, (unsigned)l3_mtu, p->pci, (unsigned)p->max_mtu, driver,
                 (unsigned)((p->max_mtu - RUSTERON_DPDK_IP_UDP_OVERHEAD) / 32 * 32));
        rusteron_dpdk_set_error(message);
        return -1;
    }
    /* Queue sizes the NIC takes (an ENA at most 256 TX descriptors under
     * large LLQ entries, the transport's default, 512 otherwise), checked
     * before anything is configured: DPDK refuses a queue outside them, and
     * the port it leaves half set up cannot be closed (see port_teardown). */
    for (int tx = 0; tx < 2; tx++)
    {
        const unsigned n = tx ? cfg->tx_descriptors : p->rx_descriptors;
        const unsigned lo = desc_lim[2 * tx], hi = desc_lim[2 * tx + 1];
        if (n < lo || (0 != hi && n > hi))
        {
            snprintf(message, sizeof(message),
                     "%s (RUSTERON_DPDK_%s) %u is outside %s's limits %u..%u (driver %s): set it within them",
                     tx ? "tx_descriptors" : "rx_descriptors", tx ? "TX_DESCRIPTORS" : "RX_DESCRIPTORS", n, p->pci,
                     lo, 0 != hi ? hi : 65535U, driver);
            rusteron_dpdk_set_error(message);
            return -1;
        }
    }
    /* A queue's free threshold must be below its ring, the receive ring as
     * this port took it (a NIC's maximum may be below what the Rust check
     * assumed). 0 keeps the PMD's default. */
    if (0 != cfg->rx_free_thresh && cfg->rx_free_thresh >= p->rx_descriptors)
    {
        snprintf(message, sizeof(message),
                 "rx_free_thresh (RUSTERON_DPDK_RX_FREE_THRESH) %u is not below %s's receive ring of %u "
                 "(driver %s): set it lower, or unset it for the PMD's default",
                 cfg->rx_free_thresh, p->pci, p->rx_descriptors, driver);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    if (0 != cfg->tx_free_thresh && cfg->tx_free_thresh >= cfg->tx_descriptors)
    {
        snprintf(message, sizeof(message),
                 "tx_free_thresh (RUSTERON_DPDK_TX_FREE_THRESH) %u is not below the transmit ring of %u: set it "
                 "lower, or unset it for the PMD's default",
                 cfg->tx_free_thresh, cfg->tx_descriptors);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    p->rx_free_thresh = cfg->rx_free_thresh;
    p->tx_free_thresh = cfg->tx_free_thresh;
    /* The pool holds both rings, four bursts and what two lcores' mempool
     * caches may keep (the port's agent, and the EAL main lcore, which starts
     * and stops the port), each up to DPDK's flush threshold of 1.5 times the
     * cache: fewer and the receive ring, refilled first, starves the sends. */
    const uint64_t caches = 3ULL * cfg->mempool_cache;
    const uint64_t need = (uint64_t)p->rx_descriptors + cfg->tx_descriptors + 4ULL * cfg->burst_size + caches;
    if ((uint64_t)cfg->mbufs_per_port < need)
    {
        snprintf(message, sizeof(message),
                 "mbufs_per_port (RUSTERON_DPDK_MBUFS_PER_PORT) %u is below the %llu %s needs: a receive ring of %u%s, "
                 "a transmit ring of %u, four bursts of %u and two mempool caches of up to %llu: raise it, or set "
                 "RUSTERON_DPDK_RX_DESCRIPTORS lower",
                 cfg->mbufs_per_port, (unsigned long long)need, p->pci, p->rx_descriptors,
                 0 == cfg->rx_descriptors ? " (RUSTERON_DPDK_RX_DESCRIPTORS unset: min(4096, the NIC's maximum))" : "",
                 cfg->tx_descriptors, cfg->burst_size, (unsigned long long)(caches / 2));
        rusteron_dpdk_set_error(message);
        return -1;
    }
    /* Checksum offloads only when the NIC offers both IPv4 and UDP (a vdev may
     * not: then both sides are done in software, never a half-done UDP sum). */
    p->tx_udp_csum_offload = p->csum_offload_ok;
    uint64_t rx_off = p->csum_offload_ok ? (RUSTERON_DPDK_RX_OFFLOAD_IPV4_CKSUM | RUSTERON_DPDK_RX_OFFLOAD_UDP_CKSUM) : 0;
    if (cfg->rx_hw_timestamp)
    {
        rx_off |= RUSTERON_DPDK_RX_OFFLOAD_TIMESTAMP;
    }
    uint64_t tx_off = p->csum_offload_ok ? (RUSTERON_DPDK_TX_OFFLOAD_IPV4_CKSUM | RUSTERON_DPDK_TX_OFFLOAD_UDP_CKSUM) : 0;
    /* Bulk frees of sent mbufs, where the NIC offers them: every frame this
     * transport sends (data, ARP, ICMP) is one segment from this port's own
     * pool, held by nothing else, which is what the PMD then assumes. */
    p->tx_fast_free = RUSTERON_DPDK_FAST_FREE_OFF;
    if (cfg->tx_fast_free)
    {
        if (p->tx_fast_free_ok)
        {
            tx_off |= RUSTERON_DPDK_TX_OFFLOAD_MBUF_FAST_FREE;
            p->tx_fast_free = RUSTERON_DPDK_FAST_FREE_ON;
        }
        else
        {
            p->tx_fast_free = RUSTERON_DPDK_FAST_FREE_UNSUPPORTED;
            fprintf(stderr,
                    "WARNING ergon-md-dpdk: %s (driver %s) does not offer TX MBUF_FAST_FREE: "
                    "RUSTERON_DPDK_TX_FAST_FREE left off on it\n",
                    p->pci, driver);
        }
    }
    /* Link-status-change interrupts where the NIC can raise them (a PMD
     * refuses the configuration otherwise): counted, never acted on. */
    p->lsc_on = NULL != ops->lsc_capable && NULL != ops->watch_lsc && ops->lsc_capable(p->port_id);
    if (ops->dev_configure(p->port_id, 1, 1, rx_off, tx_off, p->lsc_on) < 0)
    {
        snprintf(message, sizeof(message), "failed to configure queues/offloads on %s", p->pci);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    p->configured = 1;

    /* Keep the mempool name short: when a pool is split across memzones DPDK
     * names them MP_<pool>_<n>, and RTE_MEMZONE_NAMESIZE is 32 — the old
     * "rusteron_dpdk_receiver_pool" (27) produced a 32-char memzone name and
     * failed with ENAMETOOLONG on the split path (rte_mempool_populate_default). */
    char pool_name[64];
    snprintf(pool_name, sizeof(pool_name), "rusteron_%s_pool",
             p->role == RUSTERON_DPDK_ROLE_SENDER ? "sender" : "receiver");
    /* Room for the largest frame in one segment: Ethernet + L3 MTU, never less
     * than DPDK's default 2048. */
    uint16_t data_room = (uint16_t)(RUSTERON_DPDK_ETH_HDR_LEN + l3_mtu);
    if (data_room < 2048)
    {
        data_room = 2048;
    }
    p->mempool = ops->mempool_create(pool_name, cfg->mbufs_per_port, cfg->mempool_cache, data_room, p->socket_id);
    if (NULL == p->mempool)
    {
        /* The port op records the underlying rte error (rte_strerror); only
         * fall back to a generic message if the op did not set one. */
        if (RUSTERON_DPDK_ERR_OK == rusteron_dpdk_last_error_code())
        {
            snprintf(message, sizeof(message), "mempool creation failed for %s", p->pci);
            rusteron_dpdk_set_error(message);
        }
        return -1;
    }
    /* Every frame must fit one mbuf, a jumbo one included: one split over two
     * is dropped on receive (multi-segment) and a send of one is refused as
     * oversized, so a pool that gives less than asked would lose every
     * full-size frame. Read from the pool, taking none of its mbufs. */
    const uint32_t frame_max = RUSTERON_DPDK_ETH_HDR_LEN + (uint32_t)l3_mtu;
    const uint32_t room = ops->mbuf_room(p->mempool);
    if (room < frame_max)
    {
        snprintf(message, sizeof(message),
                 "%s's mbufs hold %u bytes of frame, below the %u bytes of a frame of max_aeron_mtu %zu "
                 "(Ethernet, IPv4 and UDP headers with it): it would arrive in two mbufs and be dropped; "
                 "lower RUSTERON_DPDK_MAX_AERON_MTU to at most %u",
                 p->pci, room, frame_max, cfg->max_aeron_mtu,
                 room > RUSTERON_DPDK_ETH_HDR_LEN + RUSTERON_DPDK_IP_UDP_OVERHEAD
                     ? (unsigned)((room - RUSTERON_DPDK_ETH_HDR_LEN - RUSTERON_DPDK_IP_UDP_OVERHEAD) / 32 * 32)
                     : 0U);
        rusteron_dpdk_set_error(message);
        return -1;
    }

    if (ops->rx_queue_setup(p->port_id, 0, p->rx_descriptors, p->mempool, p->socket_id, p->rx_free_thresh) < 0)
    {
        snprintf(message, sizeof(message), "rx queue setup failed on %s", p->pci);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    p->rx_queue_id = 0;

    if (ops->tx_queue_setup(p->port_id, 0, cfg->tx_descriptors, p->socket_id, p->tx_free_thresh) < 0)
    {
        snprintf(message, sizeof(message), "tx queue setup failed on %s", p->pci);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    p->tx_queue_id = 0;
    p->queues_ready = 1;

    if (ops->dev_set_mtu(p->port_id, l3_mtu) < 0)
    {
        snprintf(message, sizeof(message),
                 "device %s rejected L3 MTU %u (max_aeron_mtu %zu)", p->pci, l3_mtu, cfg->max_aeron_mtu);
        rusteron_dpdk_set_error(message);
        return -1;
    }

    /* Device-reset requests (ENA raises one when its watchdog, run by the
     * housekeeping thread's timer service, finds the device unwell), from
     * before the port starts until before it closes. */
    if (NULL != ops->watch_reset)
    {
        if (ops->watch_reset(p->port_id, &p->nic_resets) < 0)
        {
            snprintf(message, sizeof(message), "cannot watch %s (port %u) for device-reset requests", p->pci,
                     p->port_id);
            rusteron_dpdk_set_error(message);
            return -1;
        }
        p->reset_watched = 1;
    }
    if (p->lsc_on)
    {
        if (ops->watch_lsc(p->port_id, &p->nic_link_changes) < 0)
        {
            snprintf(message, sizeof(message), "cannot watch %s (port %u) for link-status changes", p->pci,
                     p->port_id);
            rusteron_dpdk_set_error(message);
            return -1;
        }
        p->lsc_watched = 1;
    }

    if (ops->dev_start(p->port_id) < 0)
    {
        snprintf(message, sizeof(message), "device %s failed to start", p->pci);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    p->started = 1;

    if (ops->link_wait_ready(p->port_id, RUSTERON_DPDK_LINK_TIMEOUT_MS) < 0)
    {
        snprintf(message, sizeof(message),
                 "link on %s (port %u) did not come up within %d ms",
                 p->pci, p->port_id, RUSTERON_DPDK_LINK_TIMEOUT_MS);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    p->link_up = 1;

    if (ena && rusteron_dpdk_ena_express_check(native, p, l3_mtu) < 0)
    {
        return -1;
    }
    if (0 == strcmp(driver, "net_netvsc") && rusteron_dpdk_netvsc_vf_check(native, p) < 0)
    {
        return -1;
    }

    /* Idle TX cleanup: one call on the empty ring, before any agent owns the
     * queue, finds whether the PMD has the op. */
    p->tx_idle_clean = 0;
    p->tx_unreaped = 0;
    p->last_tx_ns = 0;
    p->tx_idle_clean_asked = native->tx_idle_clean;
    if (0 != native->tx_idle_clean)
    {
        const int rc = NULL != ops->tx_done_cleanup ? ops->tx_done_cleanup(p->port_id, p->tx_queue_id, native->tx_idle_clean)
                                                    : -ENOTSUP;
        if (-ENOTSUP == rc)
        {
            fprintf(stderr,
                    "WARNING ergon-md-dpdk: %s (driver %s) cannot reclaim sent descriptors on demand "
                    "(rte_eth_tx_done_cleanup): RUSTERON_DPDK_TX_IDLE_CLEAN left off on it\n",
                    p->pci, p->driver);
        }
        else
        {
            p->tx_idle_clean = native->tx_idle_clean;
        }
    }
    return 0;
}

int rusteron_dpdk_parse_mac(const char *text, uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN])
{
    if (NULL == text || 17 != strlen(text))
    {
        return -1;
    }
    for (int i = 0; i < RUSTERON_DPDK_ETH_ADDR_LEN; i++)
    {
        const char *octet = text + i * 3;
        if (i < RUSTERON_DPDK_ETH_ADDR_LEN - 1 && ':' != octet[2])
        {
            return -1;
        }
        unsigned value = 0;
        for (int j = 0; j < 2; j++)
        {
            char c = octet[j];
            unsigned nibble;
            if (c >= '0' && c <= '9')
            {
                nibble = (unsigned)(c - '0');
            }
            else if (c >= 'a' && c <= 'f')
            {
                nibble = (unsigned)(c - 'a' + 10);
            }
            else if (c >= 'A' && c <= 'F')
            {
                nibble = (unsigned)(c - 'A' + 10);
            }
            else
            {
                return -1;
            }
            value = (value << 4) | nibble;
        }
        mac[i] = (uint8_t)value;
    }
    return 0;
}

/* Append " <arg>" (no leading space for the first) to out; -1 if it overflows. */
static int rusteron_dpdk_append_arg(char *out, size_t len, size_t *used, const char *arg)
{
    int n = snprintf(out + *used, len - *used, "%s%s", 0 == *used ? "" : " ", arg);
    if (n < 0 || (size_t)n >= len - *used)
    {
        return -1;
    }
    *used += (size_t)n;
    return 0;
}

int rusteron_dpdk_ena_devargs(char *out, size_t len, int *llq_off)
{
    /* DPDK 24.11 replaced ENA's enable_llq (on by default) with llq_policy,
     * whose default 24.11.0 and .1 left at 0, disabled ("net/ena: fix missing
     * default LLQ policy", 24.11.2): so the transport always names it, as
     * large 256-byte entries (3), which push a whole small frame's headers
     * with the descriptor (c9g: -0.7 us p50, -1.0 us p99 against 1, the
     * device's recommendation; BENCHMARKS.md). They halve the TX ring an ENA
     * takes, to 256 (RUSTERON_DPDK_TX_DESCRIPTORS' default). */
    int year = 0, month = 0;
    rusteron_dpdk_eal_version(&year, &month);
    const int policy = year > 24 || (24 == year && month >= 11);
    const char *key = policy ? "llq_policy" : "enable_llq";
    const size_t key_len = strlen(key);
    /* The key's value must be a decimal in the PMD's range: ENA reads
     * llq_policy with strtoul, so 0x3, 0.5 or 4294967296 would be policy 0
     * (LLQ off) with the BAR still mapped write-combined at probe. */
    const unsigned most = policy ? 3U : 1U;
    const char *extra = getenv("RUSTERON_DPDK_ENA_DEVARGS");
    if (NULL == extra)
    {
        extra = "";
    }
    *llq_off = 0;
    int named = 0;
    const size_t extra_len = strlen(extra);
    int malformed = extra_len > RUSTERON_DPDK_ENA_DEVARGS_MAX;
    for (const char *item = extra; !malformed && '\0' != *item;)
    {
        const char *end = strchr(item, ',');
        const size_t n = NULL != end ? (size_t)(end - item) : strlen(item);
        const char *eq = memchr(item, '=', n);
        malformed = NULL == eq || eq == item || eq + 1 == item + n;
        for (size_t i = 0; !malformed && i < n; i++)
        {
            const char c = item[i];
            malformed = !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') ||
                          '_' == c || '.' == c || ':' == c || '-' == c || ('=' == c && item + i == eq));
        }
        if (!malformed && (size_t)(eq - item) == key_len && 0 == strncmp(item, key, key_len))
        {
            named = 1;
            /* 0 turns LLQ off under either key */
            unsigned value = 0;
            for (const char *c = eq + 1; !malformed && c < item + n; c++)
            {
                malformed = *c < '0' || *c > '9' || (value = value * 10U + (unsigned)(*c - '0')) > most;
            }
            *llq_off = 0 == value;
        }
        item = NULL != end ? end + 1 : item + n;
        malformed = malformed || (NULL != end && '\0' == *item); /* a trailing comma */
    }
    if (malformed)
    {
        char message[384];
        snprintf(message, sizeof(message),
                 "RUSTERON_DPDK_ENA_DEVARGS must be key=value device arguments separated by commas "
                 "([A-Za-z0-9_.:-]), at most %d bytes, and %s a decimal %s, e.g. %s=1",
                 RUSTERON_DPDK_ENA_DEVARGS_MAX, key, policy ? "from 0 to 3" : "0 or 1", key);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    const int n = named ? snprintf(out, len, "%s", extra)
                        : snprintf(out, len, "%s=%u%s%s", key, policy ? 3U : 1U, '\0' != extra[0] ? "," : "", extra);
    if (n < 0 || (size_t)n >= len)
    {
        rusteron_dpdk_set_error("RUSTERON_DPDK_ENA_DEVARGS does not fit the EAL argument buffer");
        return -1;
    }
    return 0;
}

int rusteron_dpdk_ena_llq_mapping(const char *maps_path, const char *bdf)
{
    FILE *maps = fopen(maps_path, "r");
    if (NULL == maps)
    {
        return RUSTERON_DPDK_ENA_LLQ_UNKNOWN;
    }
    /* sysfs names the device in lower case; the configured BDF may not be */
    char wc[64], uc[64];
    snprintf(wc, sizeof(wc), "/%s/resource2_wc", bdf);
    snprintf(uc, sizeof(uc), "/%s/resource2", bdf);
    const size_t wc_len = strlen(wc), uc_len = strlen(uc);
    int seen_wc = 0, seen_uc = 0;
    char line[4352]; /* the fields and a PATH_MAX path */
    while (NULL != fgets(line, sizeof(line), maps))
    {
        size_t n = strlen(line);
        while (n > 0 && ('\n' == line[n - 1] || ' ' == line[n - 1]))
        {
            line[--n] = '\0';
        }
        if (n >= wc_len && 0 == strcasecmp(line + n - wc_len, wc))
        {
            seen_wc = 1;
        }
        else if (n >= uc_len && 0 == strcasecmp(line + n - uc_len, uc))
        {
            seen_uc = 1;
        }
    }
    fclose(maps);
    return seen_wc ? RUSTERON_DPDK_ENA_LLQ_WC : seen_uc ? RUSTERON_DPDK_ENA_LLQ_UNCACHED : RUSTERON_DPDK_ENA_LLQ_UNSEEN;
}

/* An option's name: `--name` of `--name` or `--name=value`, into `name`. */
static void rusteron_dpdk_eal_option_name(const char *tok, char *name, size_t len)
{
    size_t n = strcspn(tok, "=");
    if (n >= len)
    {
        n = len - 1;
    }
    memcpy(name, tok, n);
    name[n] = '\0';
}

/* Whether RUSTERON_DPDK_EAL_ARGS contradicts the memory choice `in_memory`;
 * the option is named in the error. In memory, the EAL keeps no hugepage
 * files and no shared configuration: no --huge-dir, --no-huge, a
 * --proc-type other than primary, or --huge-unlink=never. Otherwise
 * --in-memory and --no-shconf belong to RUSTERON_DPDK_EAL_IN_MEMORY. */
static int rusteron_dpdk_eal_args_conflict(int in_memory)
{
    const char *extra = getenv("RUSTERON_DPDK_EAL_ARGS");
    if (NULL == extra)
    {
        return 0;
    }
    char copy[1024];
    snprintf(copy, sizeof(copy), "%s", extra);
    char *save = NULL;
    for (char *tok = strtok_r(copy, " \t", &save); NULL != tok; tok = strtok_r(NULL, " \t", &save))
    {
        char name[64];
        rusteron_dpdk_eal_option_name(tok, name, sizeof(name));
        const char *value = strchr(tok, '=');
        int conflict = 0;
        if (in_memory)
        {
            if (0 == strcmp(name, "--proc-type"))
            {
                const char *type = NULL != value ? value + 1 : strtok_r(NULL, " \t", &save);
                conflict = NULL == type || 0 != strcmp(type, "primary");
            }
            else
            {
                conflict = 0 == strcmp(name, "--huge-dir") || 0 == strcmp(name, "--no-huge") ||
                    (0 == strcmp(name, "--huge-unlink") && NULL != value && 0 == strcmp(value + 1, "never"));
            }
        }
        else
        {
            conflict = 0 == strcmp(name, "--in-memory") || 0 == strcmp(name, "--no-shconf");
        }
        if (conflict)
        {
            char message[512];
            if (in_memory)
            {
                snprintf(message, sizeof(message),
                         "RUSTERON_DPDK_EAL_ARGS option %s contradicts RUSTERON_DPDK_EAL_IN_MEMORY=1 (--in-memory: no "
                         "hugepage files, no shared configuration, a primary process): remove it, or unset "
                         "RUSTERON_DPDK_EAL_IN_MEMORY",
                         name);
            }
            else
            {
                snprintf(message, sizeof(message),
                         "RUSTERON_DPDK_EAL_ARGS option %s changes how the EAL keeps its memory: set "
                         "RUSTERON_DPDK_EAL_IN_MEMORY=1 instead",
                         name);
            }
            rusteron_dpdk_set_error_code(message, RUSTERON_DPDK_ERR_INVALID_ENV);
            return -1;
        }
    }
    return 0;
}

int rusteron_dpdk_eal_in_memory(int *in_memory)
{
    *in_memory = 0;
    const char *v = getenv("RUSTERON_DPDK_EAL_IN_MEMORY");
    if (NULL != v && '\0' != v[0])
    {
        if (0 == strcasecmp(v, "1") || 0 == strcasecmp(v, "true"))
        {
            *in_memory = 1;
        }
        else if (0 != strcasecmp(v, "0") && 0 != strcasecmp(v, "false"))
        {
            char message[256];
            snprintf(message, sizeof(message), "RUSTERON_DPDK_EAL_IN_MEMORY=%.64s must be 0, 1, false or true", v);
            rusteron_dpdk_set_error_code(message, RUSTERON_DPDK_ERR_INVALID_ENV);
            return -1;
        }
    }
    return rusteron_dpdk_eal_args_conflict(*in_memory);
}

int rusteron_dpdk_eal_mem_args(const rusteron_dpdk_config_t *cfg, int mode, char *out, size_t len)
{
    int in_memory = 0;
    if (NULL == cfg || NULL == out || 0 == len)
    {
        rusteron_dpdk_set_error("eal_mem_args requires a config and an output buffer");
        return -1;
    }
    if (rusteron_dpdk_eal_in_memory(&in_memory) < 0)
    {
        return -1;
    }
    if (RUSTERON_DPDK_EAL_NO_HUGE == mode)
    {
        snprintf(out, len, "%s", in_memory ? "--no-huge --in-memory" : "--no-huge");
    }
    else if (in_memory)
    {
        snprintf(out, len, "--in-memory");
    }
    else
    {
        snprintf(out, len, "--huge-dir %s", cfg->hugepage_dir);
    }
    return 0;
}

int rusteron_dpdk_eal_allow_args(const rusteron_dpdk_config_t *cfg, char *out, size_t len)
{
    if (NULL == cfg || NULL == out || 0 == len)
    {
        rusteron_dpdk_set_error("eal_allow_args requires a config and an output buffer");
        return -1;
    }
    out[0] = '\0';
    size_t used = 0;
    const char *sel[2] = { cfg->sender_pci, cfg->receiver_pci };
    const char *mac[2] = { cfg->sender_mac, cfg->receiver_mac };
    /* A PCI port selected without a MAC is an ENA (MANA ports need one): its
     * device arguments name the LLQ policy (rusteron_dpdk_ena_devargs). */
    char ena[RUSTERON_DPDK_ENA_DEVARGS_MAX + 32];
    int llq_off = 0;
    if (((rusteron_dpdk_selector_is_pci(sel[0]) && '\0' == mac[0][0]) ||
         (rusteron_dpdk_selector_is_pci(sel[1]) && '\0' == mac[1][0])) &&
        rusteron_dpdk_ena_devargs(ena, sizeof(ena), &llq_off) < 0)
    {
        return -1;
    }
    for (int i = 0; i < 2; i++)
    {
        char arg[96 + sizeof(ena)];
        if (!rusteron_dpdk_selector_is_pci(sel[i]))
        {
            snprintf(arg, sizeof(arg), "--vdev=%s", sel[i]);
        }
        else if ('\0' == mac[i][0])
        {
            snprintf(arg, sizeof(arg), "%s,%s", sel[i], ena);
        }
        else if (1 == i && '\0' != mac[0][0] && 0 == strcmp(sel[0], sel[1]))
        {
            continue; /* the receiver's MAC went with the sender's -a below */
        }
        else if (0 == i && '\0' != mac[1][0] && 0 == strcmp(sel[0], sel[1]))
        {
            /* One MANA function serves both roles: one -a, both vports. */
            snprintf(arg, sizeof(arg), "%s,mac=%s,mac=%s", sel[0], mac[0], mac[1]);
        }
        else
        {
            snprintf(arg, sizeof(arg), "%s,mac=%s", sel[i], mac[i]);
        }
        if ((rusteron_dpdk_selector_is_pci(sel[i]) && rusteron_dpdk_append_arg(out, len, &used, "-a") < 0) ||
            rusteron_dpdk_append_arg(out, len, &used, arg) < 0)
        {
            rusteron_dpdk_set_error("EAL device arguments do not fit the buffer");
            return -1;
        }
    }
    /* MAC-selected ports are Azure MANA. There DPDK auto-probes vdev_netvsc,
     * whose fail-safe device takes ownership of the MANA ports (so they never
     * reach the transport) and could take the primary NIC's; an explicit
     * instance with ignore=1 stops the auto-probe. */
    if (('\0' != mac[0][0] || '\0' != mac[1][0]) &&
        rusteron_dpdk_append_arg(out, len, &used, "--vdev=net_vdev_netvsc0,ignore=1") < 0)
    {
        rusteron_dpdk_set_error("EAL device arguments do not fit the buffer");
        return -1;
    }
    /* Operator EAL options (log levels, lcore lists), whitespace-separated. */
    const char *extra = getenv("RUSTERON_DPDK_EAL_ARGS");
    if (NULL != extra)
    {
        char copy[1024]; /* as long as the EAL's whole argument buffer */
        if (strlen(extra) >= sizeof(copy))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_EAL_ARGS does not fit the EAL argument buffer");
            return -1;
        }
        snprintf(copy, sizeof(copy), "%s", extra);
        char *save = NULL;
        for (char *tok = strtok_r(copy, " \t", &save); NULL != tok; tok = strtok_r(NULL, " \t", &save))
        {
            if (rusteron_dpdk_append_arg(out, len, &used, tok) < 0)
            {
                rusteron_dpdk_set_error("RUSTERON_DPDK_EAL_ARGS does not fit the EAL argument buffer");
                return -1;
            }
        }
    }
    return 0;
}

/* "lo hi" or "lo-hi" (1 <= lo <= hi <= 65535). -1 when malformed. */
static int rusteron_dpdk_parse_port_range(const char *text, uint16_t *lo, uint16_t *hi)
{
    unsigned long a, b;
    char *end = NULL;
    a = strtoul(text, &end, 10);
    if (end == text)
    {
        return -1;
    }
    while (' ' == *end || '\t' == *end || '-' == *end)
    {
        end++;
    }
    const char *second = end;
    b = strtoul(second, &end, 10);
    while (' ' == *end || '\t' == *end || '\n' == *end)
    {
        end++;
    }
    if (end == second || '\0' != *end || a < 1 || b > 65535 || a > b)
    {
        return -1;
    }
    *lo = (uint16_t)a;
    *hi = (uint16_t)b;
    return 0;
}

static int rusteron_dpdk_read_small_file(const char *path, char *buf, size_t len)
{
    FILE *f = fopen(path, "r");
    if (NULL == f)
    {
        return -1;
    }
    size_t n = fread(buf, 1, len - 1, f);
    fclose(f);
    buf[n] = '\0';
    return 0;
}

/* "8080,9000-9010": ports the OS never hands out as ephemeral. */
static void rusteron_dpdk_parse_reserved_ports(const char *text, uint64_t *bits)
{
    const char *p = text;
    while ('\0' != *p)
    {
        char *end = NULL;
        unsigned long a = strtoul(p, &end, 10);
        if (end == p)
        {
            p++;
            continue;
        }
        unsigned long b = a;
        if ('-' == *end)
        {
            const char *q = end + 1;
            b = strtoul(q, &end, 10);
            if (end == q)
            {
                b = a;
            }
        }
        for (unsigned long port = a; port <= b && port <= 65535; port++)
        {
            bits[port / 64] |= 1ULL << (port % 64);
        }
        p = end;
    }
}

/* "10.0.0.7,10.0.0.8": more local IPv4s a NIC answers for. */
static int rusteron_dpdk_parse_extra_ips(const char *var, rusteron_dpdk_port_t *port)
{
    const char *spec = getenv(var);
    port->extra_ip_count = 0;
    if (NULL == spec || '\0' == spec[0])
    {
        return 0;
    }
    char buf[512];
    char message[640];
    if (strlen(spec) >= sizeof(buf))
    {
        snprintf(message, sizeof(message), "%s is longer than %zu bytes", var, sizeof(buf) - 1);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    snprintf(buf, sizeof(buf), "%s", spec);
    char *save = NULL;
    for (char *item = strtok_r(buf, ", ", &save); NULL != item; item = strtok_r(NULL, ", ", &save))
    {
        uint32_t ip;
        if (port->extra_ip_count >= RUSTERON_DPDK_MAX_EXTRA_IPS || rusteron_dpdk_parse_ipv4(item, &ip) < 0)
        {
            snprintf(message, sizeof(message), "%s: \"%s\" is not an IPv4 address (at most %d)",
                     var, item, RUSTERON_DPDK_MAX_EXTRA_IPS);
            rusteron_dpdk_set_error(message);
            return -1;
        }
        port->extra_ips[port->extra_ip_count++] = ip;
    }
    return 0;
}

/* Network facts the transport needs, read once before any agent runs. */
static int rusteron_dpdk_runtime_net_init(rusteron_dpdk_transport_t *native)
{
    char buf[4096];
    char message[512];

    /* The ephemeral range: explicit, else the netns's own (a pod's, under
     * Kubernetes), else Linux's default. */
    const char *range = getenv("RUSTERON_DPDK_EPHEMERAL_PORT_RANGE");
    uint16_t kernel_lo = 32768, kernel_hi = 60999;
    if (0 == rusteron_dpdk_read_small_file("/proc/sys/net/ipv4/ip_local_port_range", buf, sizeof(buf)))
    {
        rusteron_dpdk_parse_port_range(buf, &kernel_lo, &kernel_hi);
    }
    if (NULL != range && '\0' != range[0])
    {
        if (rusteron_dpdk_parse_port_range(range, &native->ephemeral_low, &native->ephemeral_high) < 0)
        {
            snprintf(message, sizeof(message),
                     "RUSTERON_DPDK_EPHEMERAL_PORT_RANGE=\"%s\" is not \"<low> <high>\" with 1 <= low <= high <= 65535",
                     range);
            rusteron_dpdk_set_error(message);
            return -1;
        }
    }
    else
    {
        native->ephemeral_low = kernel_lo;
        native->ephemeral_high = kernel_hi;
    }
    memset(native->reserved_ports, 0, sizeof(native->reserved_ports));
    if (0 == rusteron_dpdk_read_small_file("/proc/sys/net/ipv4/ip_local_reserved_ports", buf, sizeof(buf)))
    {
        rusteron_dpdk_parse_reserved_ports(buf, native->reserved_ports);
    }

    /* Start each NIC's search at a random offset, as Linux does. */
    uint32_t span = (uint32_t)(native->ephemeral_high - native->ephemeral_low) + 1;
    uint32_t r[2] = { 0, 0 };
    if (sizeof(r) != getrandom(r, sizeof(r), GRND_NONBLOCK))
    {
        r[0] = (uint32_t)rusteron_dpdk_clock_ns();
        r[1] = r[0] * 2654435761u;
    }
    native->sender_ports.ephemeral_next = (uint16_t)(native->ephemeral_low + r[0] % span);
    native->receiver_ports.ephemeral_next = (uint16_t)(native->ephemeral_low + r[1] % span);

    if (rusteron_dpdk_parse_extra_ips("RUSTERON_DPDK_SENDER_EXTRA_IPV4", &native->sender) < 0 ||
        rusteron_dpdk_parse_extra_ips("RUSTERON_DPDK_RECEIVER_EXTRA_IPV4", &native->receiver) < 0)
    {
        return -1;
    }

    /* Host-local addresses: every kernel interface's IPv4 except the DPDK
     * NICs'. A kernel interface that also holds a DPDK NIC's IP shares its
     * port space with the transport: then the transport's ephemeral range
     * must not overlap the kernel's unless every port of it is reserved. */
    /* RUSTERON_DPDK_HOST_LOCAL: "all" (default) treats loopback and every
     * kernel interface's IP as host-local; "loopback" only 127/8 (e.g. a test
     * rig whose DPDK peer is a kernel interface on the same host). */
    /* How agents read host-local kernel sockets: `adaptive` (default) only
     * when the local-I/O thread flags data while a socket is idle, but every duty cycle for RUSTERON_DPDK_LOCAL_HOT_US (default
     * 1000) after it last had data, so an active local channel does not wait
     * for a thread wake-up per message; `spin` every duty cycle always (a
     * recvmmsg per socket per cycle on the agent, idle or not); `epoll` only
     * ever on the flag (a thread wake-up per local message). Measured on
     * Azure: adaptive halves the localhost round trip of epoll (p50 6.4 vs
     * 14.1 us, max 61 vs 260 us) at no cost while idle. */
    const char *local_io = getenv("RUSTERON_DPDK_LOCAL_IO");
    native->local_io_mode = RUSTERON_DPDK_LOCAL_IO_ADAPTIVE;
    if (NULL != local_io && '\0' != local_io[0])
    {
        if (0 == strcmp(local_io, "spin"))
        {
            native->local_io_mode = RUSTERON_DPDK_LOCAL_IO_SPIN;
        }
        else if (0 == strcmp(local_io, "adaptive"))
        {
            native->local_io_mode = RUSTERON_DPDK_LOCAL_IO_ADAPTIVE;
        }
        else if (0 == strcmp(local_io, "epoll"))
        {
            /* Every explicit choice assigns its own mode: "epoll" used to fall
             * through and leave the default ADAPTIVE, so an explicit epoll run
             * measured adaptive and the two modes were not comparable. */
            native->local_io_mode = RUSTERON_DPDK_LOCAL_IO_EPOLL;
        }
        else
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_LOCAL_IO must be epoll, adaptive or spin");
            return -1;
        }
    }
    /* How often housekeeping reads the NIC's own statistics (on ENA each read
     * is an admin command to the device): every second by default, 0 never. */
    native->nic_stats_ns = 1000000000ULL;
    const char *nic_stats = getenv("RUSTERON_DPDK_NIC_STATS_MS");
    if (NULL != nic_stats && '\0' != nic_stats[0])
    {
        char *end = NULL;
        const unsigned long long ms = strtoull(nic_stats, &end, 10);
        if ('\0' != *end || '-' == nic_stats[0] || (0 != ms && (ms < 1000ULL || ms > 3600000ULL)))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_NIC_STATS_MS must be 0 (never) or milliseconds from 1000 to 3600000");
            return -1;
        }
        native->nic_stats_ns = ms * 1000000ULL;
    }
    /* Split the NIC reads: every RUSTERON_DPDK_NIC_STATS_MS only the PMD's
     * software xstats (no admin command on ENA), and everything (basic
     * statistics, every xstat) every RUSTERON_DPDK_NIC_SLOW_STATS_MS, 0
     * never. Unset: every read is a full one. */
    native->nic_slow_stats_ns = RUSTERON_DPDK_NIC_STATS_UNSPLIT;
    const char *nic_slow = getenv("RUSTERON_DPDK_NIC_SLOW_STATS_MS");
    if (NULL != nic_slow && '\0' != nic_slow[0])
    {
        char *end = NULL;
        const unsigned long long ms = strtoull(nic_slow, &end, 10);
        if ('\0' != *end || '-' == nic_slow[0] || (0 != ms && (ms < 1000ULL || ms > 3600000ULL)))
        {
            rusteron_dpdk_set_error(
                "RUSTERON_DPDK_NIC_SLOW_STATS_MS must be 0 (never) or milliseconds from 1000 to 3600000");
            return -1;
        }
        native->nic_slow_stats_ns = ms * 1000000ULL;
    }
    /* Time the PMD bursts and split poll-cycle gaps into work and
     * descheduling: off by default, so the hot path never reads the thread
     * CPU clock for it. `burst` times the bursts alone: the CPU clock is a
     * system call every poll (no vDSO path), which changes the loop. */
    rusteron_dpdk_gap_attrib = 0;
    const char *gap_attrib = getenv("RUSTERON_DPDK_GAP_ATTRIB");
    if (NULL != gap_attrib && '\0' != gap_attrib[0])
    {
        if (0 == strcmp(gap_attrib, "1"))
        {
            rusteron_dpdk_gap_attrib = RUSTERON_DPDK_GAP_ATTRIB_BURST | RUSTERON_DPDK_GAP_ATTRIB_CPU;
        }
        else if (0 == strcmp(gap_attrib, "burst"))
        {
            rusteron_dpdk_gap_attrib = RUSTERON_DPDK_GAP_ATTRIB_BURST;
        }
        else if (0 != strcmp(gap_attrib, "0"))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_GAP_ATTRIB must be 0, 1 or burst");
            return -1;
        }
    }
    /* Reclaim sent descriptors on an idle poll, at most this many a poll,
     * instead of inside the send that crosses the PMD's free threshold: off
     * by default. */
    native->tx_idle_clean = 0;
    const char *idle_clean = getenv("RUSTERON_DPDK_TX_IDLE_CLEAN");
    if (NULL != idle_clean && '\0' != idle_clean[0])
    {
        char *end = NULL;
        const unsigned long long n = strtoull(idle_clean, &end, 10);
        if ('\0' != *end || '-' == idle_clean[0] || n > RUSTERON_DPDK_TX_IDLE_CLEAN_MAX)
        {
            rusteron_dpdk_set_error(
                "RUSTERON_DPDK_TX_IDLE_CLEAN must be 0 (off) or the descriptors an idle poll reclaims, 1 to 1024");
            return -1;
        }
        native->tx_idle_clean = (uint32_t)n;
    }
    /* How many bursts one poll drains while they come back full: by default
     * as many as fill the receive ring, at most 8, so a backlog a stall left
     * clears in one duty cycle; 0 until the ports, whose rings it sizes from,
     * are up (rusteron_dpdk_runtime_init). */
    native->rx_max_bursts = 0;
    const char *max_bursts = getenv("RUSTERON_DPDK_RX_MAX_BURSTS");
    if (NULL != max_bursts && '\0' != max_bursts[0])
    {
        char *end = NULL;
        const unsigned long long n = strtoull(max_bursts, &end, 10);
        if ('\0' != *end || '-' == max_bursts[0] || n < 1 || n > 64)
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_RX_MAX_BURSTS must be a number of bursts from 1 to 64");
            return -1;
        }
        native->rx_max_bursts = (uint32_t)n;
    }
    native->tx_retry_ns = 0;
    const char *tx_retry = getenv("RUSTERON_DPDK_TX_RETRY_US");
    if (NULL != tx_retry && '\0' != tx_retry[0])
    {
        char *end = NULL;
        const unsigned long long us = strtoull(tx_retry, &end, 10);
        if ('\0' != *end || us > 1000000ULL)
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_TX_RETRY_US must be microseconds, at most 1000000");
            return -1;
        }
        native->tx_retry_ns = us * 1000ULL;
    }
    /* The bandwidth allowance per NIC, megabits per second of frames (unset
     * or 0: none, the fastest path), its bucket
     * (RUSTERON_DPDK_TX_BUCKET_BYTES, default 64 KiB) and the longest wait to
     * repay a send's overdraft (default 20 us). */
    uint64_t rate = 0, bucket = 65536, pace_spin_us = 20;
    const struct { const char *name; uint64_t *value; uint64_t max; } pacing[] = {
        { "RUSTERON_DPDK_TX_RATE_MBPS", &rate, 400000ULL },
        { "RUSTERON_DPDK_TX_BUCKET_BYTES", &bucket, 1ULL << 30 },
        { "RUSTERON_DPDK_TX_PACE_SPIN_US", &pace_spin_us, 1000000ULL },
    };
    for (size_t i = 0; i < sizeof(pacing) / sizeof(pacing[0]); i++)
    {
        const char *v = getenv(pacing[i].name);
        if (NULL != v && '\0' != v[0])
        {
            char *end = NULL;
            const unsigned long long n = strtoull(v, &end, 10);
            if ('\0' != *end || n > pacing[i].max)
            {
                char msg[128];
                snprintf(msg, sizeof(msg), "%s must be a number, at most %llu", pacing[i].name,
                         (unsigned long long)pacing[i].max);
                rusteron_dpdk_set_error(msg);
                return -1;
            }
            *pacing[i].value = n;
        }
    }
    native->tx_pace_spin_ns = pace_spin_us * 1000ULL;
    const char *pace_spin_env = getenv("RUSTERON_DPDK_TX_PACE_SPIN_US");
    native->tx_pace_spin_set = NULL != pace_spin_env && '\0' != pace_spin_env[0];
    /* An allowance the operator set is a link's capacity to share out (and the
     * only thing that can put low-priority streams below the rest). Unset or
     * 0 arms none: the send path skips the whole block. */
    const char *rate_env = getenv("RUSTERON_DPDK_TX_RATE_MBPS");
    const int rate_set = NULL != rate_env && '\0' != rate_env[0] && 0 != rate;
    /* RUSTERON_DPDK_TX_ROUTE_MBPS="<a.b.c.d>/<len>=<Mbit/s>,...": a slower link
     * on the way to those destinations (a direct link to another site, say),
     * each with its own allowance; the longest matching prefix applies. */
    uint64_t route_rate[RUSTERON_DPDK_MAX_TX_ROUTES];
    uint32_t route_len[RUSTERON_DPDK_MAX_TX_ROUTES];
    native->tx_route_count = 0;
    const char *routes = getenv("RUSTERON_DPDK_TX_ROUTE_MBPS");
    if (NULL != routes && '\0' != routes[0])
    {
        char list[1024];
        if (strlen(routes) >= sizeof(list))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_TX_ROUTE_MBPS is too long");
            return -1;
        }
        strcpy(list, routes);
        char *save = NULL;
        for (char *item = strtok_r(list, ",", &save); NULL != item; item = strtok_r(NULL, ",", &save))
        {
            char ip_text[32];
            unsigned int len = 0;
            unsigned long long mbps = 0;
            int consumed = 0;
            uint32_t ip = 0;
            if (native->tx_route_count == RUSTERON_DPDK_MAX_TX_ROUTES ||
                3 != sscanf(item, " %31[0-9.]/%u=%llu %n", ip_text, &len, &mbps, &consumed) ||
                '\0' != item[consumed] || len > 32 || 0 == mbps || mbps > 400000ULL ||
                rusteron_dpdk_parse_ipv4(ip_text, &ip) < 0)
            {
                rusteron_dpdk_set_error(
                    "RUSTERON_DPDK_TX_ROUTE_MBPS must be up to 16 <a.b.c.d>/<len>=<Mbit/s>, comma-separated");
                return -1;
            }
            /* insert keeping the longest prefixes first */
            uint32_t at = native->tx_route_count;
            while (at > 0 && route_len[at - 1] < len)
            {
                native->tx_route_net[at] = native->tx_route_net[at - 1];
                native->tx_route_mask[at] = native->tx_route_mask[at - 1];
                route_rate[at] = route_rate[at - 1];
                route_len[at] = route_len[at - 1];
                at--;
            }
            const uint32_t mask = 0 == len ? 0 : rusteron_dpdk_ipv4_mask((uint8_t)len); /* /0: every destination */
            native->tx_route_mask[at] = mask;
            native->tx_route_net[at] = ip & mask;
            route_rate[at] = mbps;
            route_len[at] = len;
            native->tx_route_count++;
        }
    }
    /* RUSTERON_DPDK_TX_LOW_STREAMS="<stream>|<first>-<last>,...": streams that
     * can wait (bulk), held below the rest under the allowances; they leave
     * RUSTERON_DPDK_TX_LOW_RESERVE_BYTES (default half the bucket) to the rest. */
    native->tx_low_count = 0;
    native->tx_low_reserve = (int64_t)(bucket / 2);
    const char *reserve = getenv("RUSTERON_DPDK_TX_LOW_RESERVE_BYTES");
    if (NULL != reserve && '\0' != reserve[0])
    {
        char *end = NULL;
        const unsigned long long n = strtoull(reserve, &end, 10);
        if ('\0' != *end || '-' == reserve[0] || n >= bucket)
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_TX_LOW_RESERVE_BYTES must be bytes, less than the bucket "
                                    "(RUSTERON_DPDK_TX_BUCKET_BYTES)");
            return -1;
        }
        native->tx_low_reserve = (int64_t)n;
    }
    const char *low = getenv("RUSTERON_DPDK_TX_LOW_STREAMS");
    if (NULL != low && '\0' != low[0])
    {
        char list[1024];
        if (strlen(low) >= sizeof(list))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_TX_LOW_STREAMS is too long");
            return -1;
        }
        strcpy(list, low);
        char *save = NULL;
        for (char *item = strtok_r(list, ",", &save); NULL != item; item = strtok_r(NULL, ",", &save))
        {
            unsigned long long first = 0, last = 0;
            int consumed = 0;
            const char *digits = item + strspn(item, " ");
            int ok = native->tx_low_count < RUSTERON_DPDK_MAX_TX_LOW && *digits >= '0' && *digits <= '9';
            if (ok && 2 == sscanf(item, " %llu - %llu %n", &first, &last, &consumed) && '\0' == item[consumed])
            {
                /* a range */
            }
            else if (consumed = 0, ok && 1 == sscanf(item, " %llu %n", &first, &consumed) && '\0' == item[consumed])
            {
                last = first;
            }
            else
            {
                ok = 0;
            }
            if (!ok || first > last || last > INT32_MAX)
            {
                rusteron_dpdk_set_error("RUSTERON_DPDK_TX_LOW_STREAMS must be up to 16 stream ids or <first>-<last> "
                                        "ranges (0 to 2147483647), comma-separated");
                return -1;
            }
            native->tx_low_first[native->tx_low_count] = (int32_t)first;
            native->tx_low_last[native->tx_low_count] = (int32_t)last;
            native->tx_low_count++;
        }
        if (!rate_set && 0 == native->tx_route_count)
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_TX_LOW_STREAMS needs an allowance to hold them back: set "
                                    "RUSTERON_DPDK_TX_RATE_MBPS or RUSTERON_DPDK_TX_ROUTE_MBPS");
            return -1;
        }
    }
    const uint64_t pacer_now = rusteron_dpdk_clock_now();
    rusteron_dpdk_port_t *paced[2] = { &native->sender, &native->receiver };
    for (int p = 0; p < 2; p++)
    {
        rusteron_dpdk_pacer_init(&paced[p]->tx_pacer, rate, (int64_t)bucket, pacer_now);
        for (uint32_t r = 0; r < native->tx_route_count; r++)
        {
            rusteron_dpdk_pacer_init(&paced[p]->route_pacers[r], route_rate[r], (int64_t)bucket, pacer_now);
        }
    }
    /* RUSTERON_DPDK_LOCAL_UNWATCH=0 keeps hot sockets on the local-I/O
     * thread's epoll (adaptive); 1, the default, takes them off. */
    native->local_unwatch = 1;
    const char *unwatch = getenv("RUSTERON_DPDK_LOCAL_UNWATCH");
    if (NULL != unwatch && '\0' != unwatch[0])
    {
        if (0 != strcmp(unwatch, "0") && 0 != strcmp(unwatch, "1"))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_LOCAL_UNWATCH must be 0 or 1");
            return -1;
        }
        native->local_unwatch = '1' == unwatch[0];
    }
    native->local_hot_ns = 1000000ULL;
    const char *hot = getenv("RUSTERON_DPDK_LOCAL_HOT_US");
    if (NULL != hot && '\0' != hot[0])
    {
        char *end = NULL;
        const unsigned long long us = strtoull(hot, &end, 10);
        if ('\0' != *end || us > 60000000ULL)
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_LOCAL_HOT_US must be microseconds, at most 60000000");
            return -1;
        }
        native->local_hot_ns = us * 1000ULL;
    }
    const char *host_local = getenv("RUSTERON_DPDK_HOST_LOCAL");
    int loopback_only = 0;
    if (NULL != host_local && '\0' != host_local[0])
    {
        if (0 == strcmp(host_local, "loopback"))
        {
            loopback_only = 1;
        }
        else if (0 != strcmp(host_local, "all"))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_HOST_LOCAL must be all or loopback");
            return -1;
        }
    }
    free(native->host_ips);
    native->host_ips = NULL;
    native->host_ip_count = 0;
    int kernel_shares_ip = 0;
    struct ifaddrs *ifs = NULL;
    if (0 == getifaddrs(&ifs))
    {
        uint32_t capacity = 0;
        for (struct ifaddrs *i = ifs; NULL != i; i = i->ifa_next)
        {
            capacity++;
        }
        native->host_ips = calloc(capacity + 1, sizeof(uint32_t));
        if (NULL == native->host_ips)
        {
            freeifaddrs(ifs);
            rusteron_dpdk_set_error("out of memory reading the host's addresses");
            return -1;
        }
        for (struct ifaddrs *i = ifs; NULL != i; i = i->ifa_next)
        {
            if (NULL == i->ifa_addr || AF_INET != i->ifa_addr->sa_family)
            {
                continue;
            }
            uint32_t ip = ((struct sockaddr_in *)i->ifa_addr)->sin_addr.s_addr;
            if (rusteron_dpdk_port_owns_ip(&native->sender, ip) || rusteron_dpdk_port_owns_ip(&native->receiver, ip))
            {
                kernel_shares_ip = 1;
                continue;
            }
            if (!loopback_only && 0x7f != ((const uint8_t *)&ip)[0])
            {
                native->host_ips[native->host_ip_count++] = __builtin_bswap32(ip);
            }
        }
        freeifaddrs(ifs);
        /* Sorted, for the binary search in rusteron_dpdk_is_host_local. */
        for (uint32_t a = 1; a < native->host_ip_count; a++)
        {
            uint32_t v = native->host_ips[a];
            uint32_t b = a;
            while (b > 0 && native->host_ips[b - 1] > v)
            {
                native->host_ips[b] = native->host_ips[b - 1];
                b--;
            }
            native->host_ips[b] = v;
        }
    }
    if (kernel_shares_ip && native->ephemeral_low <= kernel_hi && kernel_lo <= native->ephemeral_high)
    {
        for (uint32_t p = native->ephemeral_low; p <= native->ephemeral_high; p++)
        {
            if (p >= kernel_lo && p <= kernel_hi && 0 == (native->reserved_ports[p / 64] & (1ULL << (p % 64))))
            {
                snprintf(message, sizeof(message),
                         "a kernel interface shares a DPDK NIC's IP and the transport's ephemeral ports %u-%u overlap "
                         "the kernel's %u-%u: set RUSTERON_DPDK_EPHEMERAL_PORT_RANGE outside it, or reserve the range "
                         "in net.ipv4.ip_local_reserved_ports",
                         native->ephemeral_low, native->ephemeral_high, kernel_lo, kernel_hi);
                rusteron_dpdk_set_error(message);
                return -1;
            }
        }
    }

    /* An ENA port whose low-latency queue is not verified fails start unless
     * RUSTERON_DPDK_ENA_LLQ_REQUIRED=0 (or false), for diagnosis. */
    native->ena_llq_required = 1;
    const char *llq_required = getenv("RUSTERON_DPDK_ENA_LLQ_REQUIRED");
    if (NULL != llq_required && '\0' != llq_required[0])
    {
        const int on = 0 == strcmp(llq_required, "1") || 0 == strcmp(llq_required, "true");
        if (!on && 0 != strcmp(llq_required, "0") && 0 != strcmp(llq_required, "false"))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_ENA_LLQ_REQUIRED must be 0, 1, false or true");
            return -1;
        }
        native->ena_llq_required = on;
    }
    /* A netvsc port without its accelerated-networking VF fails start unless
     * RUSTERON_DPDK_ALLOW_SYNTHETIC=1 (or true), for diagnosis only. */
    native->allow_synthetic = 0;
    const char *allow_synthetic = getenv("RUSTERON_DPDK_ALLOW_SYNTHETIC");
    if (NULL != allow_synthetic && '\0' != allow_synthetic[0])
    {
        const int on = 0 == strcmp(allow_synthetic, "1") || 0 == strcmp(allow_synthetic, "true");
        if (!on && 0 != strcmp(allow_synthetic, "0") && 0 != strcmp(allow_synthetic, "false"))
        {
            rusteron_dpdk_set_error_code(
                "RUSTERON_DPDK_ALLOW_SYNTHETIC must be 0, 1, false or true (1: start an Azure netvsc port without "
                "its accelerated-networking VF, for diagnosis only)",
                RUSTERON_DPDK_ERR_INVALID_ENV);
            return -1;
        }
        native->allow_synthetic = on;
    }
    /* RUSTERON_DPDK_VFIO_WC=1: a write-combining vfio-pci, as preflight reads
     * it (0, the default, or 1). */
    native->vfio_wc = 0;
    const char *vfio_wc = getenv("RUSTERON_DPDK_VFIO_WC");
    if (NULL != vfio_wc && '\0' != vfio_wc[0])
    {
        native->vfio_wc = 0 == strcmp(vfio_wc, "1");
        if (!native->vfio_wc && 0 != strcmp(vfio_wc, "0"))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_VFIO_WC must be 0 or 1 (1: the ports' vfio-pci maps BARs "
                                    "write-combined, as the node's preflight confirmed)");
            return -1;
        }
    }
    /* Malformed ENA device arguments fail here whatever the ports are. */
    char ena_devargs[RUSTERON_DPDK_ENA_DEVARGS_MAX + 32];
    int llq_off = 0;
    if (rusteron_dpdk_ena_devargs(ena_devargs, sizeof(ena_devargs), &llq_off) < 0)
    {
        return -1;
    }

    /* A 0.0.0.0 receive bind also listens on the host's own interfaces unless
     * RUSTERON_DPDK_WILDCARD_LOCAL=0 (or false): it costs nothing while idle. */
    native->wildcard_local = 1;
    const char *wildcard = getenv("RUSTERON_DPDK_WILDCARD_LOCAL");
    if (NULL != wildcard && '\0' != wildcard[0])
    {
        const int on = 0 == strcmp(wildcard, "1") || 0 == strcmp(wildcard, "true");
        if (!on && 0 != strcmp(wildcard, "0") && 0 != strcmp(wildcard, "false"))
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_WILDCARD_LOCAL must be 0, 1, false or true");
            return -1;
        }
        native->wildcard_local = on;
    }
    return 0;
}

/* Aeron's distinct error log, from the driver library when it is linked. */
extern int aeron_distinct_error_log_record(aeron_distinct_error_log_t *log, int error_code, const char *description)
    __attribute__((weak));

/* An error housekeeping found, which no Aeron call returns, goes to standard
 * error at error level, with this prefix. */
#define RUSTERON_DPDK_LOG_ERROR "ERROR ergon-md-dpdk: "
static void rusteron_dpdk_housekeeping_log(const char *message)
{
    fprintf(stderr, RUSTERON_DPDK_LOG_ERROR "%s\n", message);
}

/* How often the PMDs' timers are run: AWS's recommended cadence for
 * rte_timer_manage with the ENA PMD, whose watchdog timer is periodic at a
 * second and checks no more often. */
#define RUSTERON_DPDK_TIMER_SERVICE_NS 1000000000ULL

/* Run the PMDs' due DPDK timers, at most once a second. ENA's watchdog is
 * one (device keep-alive, admin queue, missing transmit completions; a
 * failed check asks for a device reset), and nothing else runs it. The
 * service needs an lcore id, so this thread registers with DPDK first (a
 * no-op once it has). A failure is reported once, until it next succeeds. */
static void rusteron_dpdk_timer_service(rusteron_dpdk_transport_t *native, uint64_t now_ns)
{
    if (NULL == native->ops->timer_service ||
        (0 != native->timer_service_ns && now_ns >= native->timer_service_ns &&
         now_ns - native->timer_service_ns < RUSTERON_DPDK_TIMER_SERVICE_NS))
    {
        return;
    }
    native->timer_service_ns = now_ns;
    const int registered = rusteron_dpdk_eal_thread_register() >= 0;
    const uint64_t start_ns = NULL != native->housekeeping_events ? rusteron_dpdk_clock_now() : 0;
    const int rc = registered ? native->ops->timer_service() : -1;
    if (NULL != native->housekeeping_events)
    {
        rusteron_dpdk_event_record(native->housekeeping_events, RUSTERON_DPDK_EVENT_TIMERS, RUSTERON_DPDK_EVENT_NO_PORT,
                                   rusteron_dpdk_clock_now() - start_ns);
    }
    if (rc >= 0)
    {
        native->timer_service_failing = 0;
        return;
    }
    if (!native->timer_service_failing)
    {
        native->timer_service_failing = 1;
        char message[384];
        snprintf(message, sizeof(message),
                 "DPDK transport: the PMDs' timer service (ENA's health checks: device keep-alive, admin queue, "
                 "missing transmit completions) %s (rc %d); retried every second",
                 registered ? "failed" : "cannot run: the housekeeping thread cannot register with DPDK", rc);
        rusteron_dpdk_housekeeping_log(message);
    }
}

/* Device resets `p`'s PMD asked for since the last pass: kept on the port's
 * counter (while a transport is open) and in the driver's distinct error log,
 * and returned for logging once the lock is released. Caller holds
 * housekeeping_lock. The transport does not reset the device: its agents
 * poll the queues a reset would rebuild. The caller sets nic_reset_exit so
 * the media driver stops those agents and exits. */
static uint64_t rusteron_dpdk_reset_requests(rusteron_dpdk_port_t *p, char *message, size_t len)
{
    const uint64_t resets = __atomic_load_n(&p->nic_resets, __ATOMIC_RELAXED);
    if (p->open_transports > 0 && NULL != p->counters.addr[RD_COUNTER_NIC_RESET_REQUESTED])
    {
        __atomic_store_n(p->counters.addr[RD_COUNTER_NIC_RESET_REQUESTED], (int64_t)resets, __ATOMIC_RELAXED);
    }
    if (resets == p->nic_resets_seen)
    {
        return 0;
    }
    const uint64_t fresh = resets - p->nic_resets_seen;
    p->nic_resets_seen = resets;
    snprintf(message, len,
             "DPDK transport: NIC %s (port %u, %s) asked for a device reset (RTE_ETH_EVENT_INTR_RESET): its PMD's "
             "health checks failed (ENA: device keep-alive, admin queue or missing transmit completions) and its "
             "queues may have stopped; the transport does not reset the device under its polling agents; the media "
             "driver stops them and exits so it can be restarted (counter nic-reset-requested)",
             p->pci, (unsigned)p->port_id, '\0' != p->driver[0] ? p->driver : "-");
    if (p->open_transports > 0 && NULL != p->error_log && NULL != aeron_distinct_error_log_record)
    {
        aeron_distinct_error_log_record(p->error_log, EIO, message);
    }
    return fresh;
}

/* `p`'s queued receive-path error report, if any: formatted into `message`
 * and recorded in the driver's distinct error log (under one fixed
 * description per kind) while a transport is open. Caller holds
 * housekeeping_lock. 1 when there was one, to print once the lock is
 * released. */
static int rusteron_dpdk_report(const rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *p, char *message,
                                size_t len)
{
    const char *description = NULL;
    if (RUSTERON_DPDK_REPORT_NONE == rusteron_dpdk_report_take(native, p, message, len, &description))
    {
        return 0;
    }
    if (p->open_transports > 0 && NULL != p->error_log && NULL != aeron_distinct_error_log_record)
    {
        aeron_distinct_error_log_record(p->error_log, EMSGSIZE, description);
    }
    return 1;
}

/* Link-status changes `p`'s PMD raised: kept on the port's counter while a
 * transport is open; the number new since the last pass is returned, to log
 * once the lock is released. Caller holds housekeeping_lock. */
static uint64_t rusteron_dpdk_link_changes(rusteron_dpdk_port_t *p)
{
    const uint64_t changes = __atomic_load_n(&p->nic_link_changes, __ATOMIC_RELAXED);
    if (p->open_transports > 0 && NULL != p->counters.addr[RD_COUNTER_NIC_LINK_CHANGES])
    {
        __atomic_store_n(p->counters.addr[RD_COUNTER_NIC_LINK_CHANGES], (int64_t)changes, __ATOMIC_RELAXED);
    }
    const uint64_t fresh = changes - p->nic_link_changes_seen;
    p->nic_link_changes_seen = changes;
    return fresh;
}

void rusteron_dpdk_housekeeping_tick(rusteron_dpdk_transport_t *native, uint64_t now_ns)
{
    rusteron_dpdk_timer_service(native, now_ns);

    rusteron_dpdk_port_t *ports[2] = { &native->sender, &native->receiver };
    char messages[2][512];
    char reports[2][384];
    int reported[2];
    uint64_t fresh[2];
    uint64_t link_changes[2];
    rusteron_dpdk_nic_sample_t nic[2];
    /* Only ports with open transports: counters belong to transports'
     * lifetimes. The lock is held to copy and to store, never across a read
     * of the NIC (on ENA an admin command to the device), so a transport's
     * close never waits behind one. */
    pthread_mutex_lock(&native->housekeeping_lock);
    for (int i = 0; i < 2; i++)
    {
        rusteron_dpdk_port_t *p = ports[i];
        fresh[i] = rusteron_dpdk_reset_requests(p, messages[i], sizeof(messages[i]));
        link_changes[i] = rusteron_dpdk_link_changes(p);
        reported[i] = rusteron_dpdk_report(native, p, reports[i], sizeof(reports[i]));
        nic[i].due = 0;
        if (p->open_transports > 0)
        {
            rusteron_dpdk_counters_sample(&p->counters, p, native->ops, now_ns, native->nic_stats_ns,
                                          native->nic_slow_stats_ns, native->housekeeping_events, &nic[i]);
        }
    }
    if (0 != fresh[0] || 0 != fresh[1])
    {
        __atomic_store_n(&native->nic_reset_exit, 1, __ATOMIC_RELEASE);
    }
    pthread_mutex_unlock(&native->housekeeping_lock);

    if (nic[0].due || nic[1].due)
    {
        for (int i = 0; i < 2; i++)
        {
            rusteron_dpdk_counters_read_nic(native->ops, &nic[i], native->housekeeping_events);
        }
        pthread_mutex_lock(&native->housekeeping_lock);
        for (int i = 0; i < 2; i++)
        {
            if (ports[i]->open_transports > 0)
            {
                rusteron_dpdk_counters_store_nic(&ports[i]->counters, &nic[i]);
            }
        }
        pthread_mutex_unlock(&native->housekeeping_lock);
    }

    for (int i = 0; i < 2; i++)
    {
        if (0 != fresh[i])
        {
            fprintf(stderr, RUSTERON_DPDK_LOG_ERROR "%s [%llu new]\n", messages[i], (unsigned long long)fresh[i]);
        }
        if (reported[i])
        {
            rusteron_dpdk_housekeeping_log(reports[i]);
        }
        if (0 != link_changes[i])
        {
            fprintf(stderr,
                    "WARNING ergon-md-dpdk: NIC %s (port %u) link status changed %llu time(s) (counter "
                    "nic-link-changes): frames sent or due while the link was down are lost\n",
                    ports[i]->pci, (unsigned)ports[i]->port_id, (unsigned long long)link_changes[i]);
        }
    }
    /* The event log's rings into its file, outside the lock. */
    rusteron_dpdk_events_drain(native);
}

int rusteron_dpdk_transport_nic_reset_exit(rusteron_dpdk_transport_t *transport)
{
    if (NULL == transport)
    {
        return 0;
    }
    return __atomic_load_n(&transport->nic_reset_exit, __ATOMIC_ACQUIRE) ? 1 : 0;
}

void rusteron_dpdk_test_housekeeping_tick(rusteron_dpdk_transport_t *native)
{
    rusteron_dpdk_housekeeping_tick(native, rusteron_dpdk_clock_now());
}

/* Every 100 ms: the 1 Hz gates inside the sample and the timer service keep
 * their rates. Pinned to the housekeeping CPU, away from the agents' cores.
 * The timer service registers the thread with DPDK; its lcore goes back as
 * it ends. */
static void *rusteron_dpdk_housekeeping_main(void *arg)
{
    rusteron_dpdk_transport_t *native = arg;
    const struct timespec step = { .tv_sec = 0, .tv_nsec = 100000000L };
    while (!__atomic_load_n(&native->housekeeping_stop, __ATOMIC_ACQUIRE))
    {
        rusteron_dpdk_housekeeping_tick(native, rusteron_dpdk_clock_now());
        nanosleep(&step, NULL);
    }
    rusteron_dpdk_eal_thread_unregister();
    return NULL;
}

/* A CPU Aeron pins one of its agents to (AERON_*_CPU_AFFINITY), else -1. */
static int rusteron_dpdk_agent_cpu(const char *name)
{
    const char *env = getenv(name);
    if (NULL == env || '\0' == env[0])
    {
        return -1;
    }
    char *end = NULL;
    long v = strtol(env, &end, 10);
    return '\0' == *end && v >= 0 && v < CPU_SETSIZE ? (int)v : -1;
}

/* AERON_THREADING_MODE as Aeron reads it (exact names; anything else is
 * DEDICATED, Aeron's default). */
static int rusteron_dpdk_threading_mode_env(void)
{
    const char *mode = getenv("AERON_THREADING_MODE");
    if (NULL == mode)
    {
        return RUSTERON_DPDK_THREADING_DEDICATED;
    }
    if (0 == strcmp(mode, "SHARED_NETWORK"))
    {
        return RUSTERON_DPDK_THREADING_SHARED_NETWORK;
    }
    if (0 == strcmp(mode, "SHARED"))
    {
        return RUSTERON_DPDK_THREADING_SHARED;
    }
    if (0 == strcmp(mode, "INVOKER"))
    {
        return RUSTERON_DPDK_THREADING_INVOKER;
    }
    return RUSTERON_DPDK_THREADING_DEDICATED;
}

/* Whether an idle strategy name busy-spins (Aeron's names and aliases). */
static int rusteron_dpdk_idle_spins(const char *name)
{
    return NULL != name &&
        (0 == strcmp(name, "spin") || 0 == strcmp(name, "noop") ||
         0 == strcmp(name, "aeron_idle_strategy_busy_spinning") || 0 == strcmp(name, "aeron_idle_strategy_noop"));
}

/* The CPUs in `set` as "0,3,5", into `buf`, which it returns. For error
 * messages: a pin that did not land has to say which CPUs it wanted. */
const char *rusteron_dpdk_cpu_set_list(const cpu_set_t *set, char *buf, size_t len)
{
    if (0 == len)
    {
        return buf;
    }
    size_t at = 0;
    buf[0] = '\0';
    for (int cpu = 0; cpu < CPU_SETSIZE && at + 1 < len; cpu++)
    {
        if (!CPU_ISSET(cpu, set))
        {
            continue;
        }
        const int wrote = snprintf(buf + at, len - at, "%s%d", at > 0 ? "," : "", cpu);
        if (wrote <= 0)
        {
            break;
        }
        at += (size_t)wrote;
    }
    return buf;
}

/* Clear `cpu` and its SMT siblings from `set`: a busy-spinning agent owns
 * its whole core. */
static void rusteron_dpdk_clear_core(cpu_set_t *set, int cpu)
{
    if (cpu < 0 || cpu >= CPU_SETSIZE)
    {
        return;
    }
    CPU_CLR(cpu, set);
    char path[96];
    snprintf(path, sizeof(path), "/sys/devices/system/cpu/cpu%d/topology/thread_siblings_list", cpu);
    FILE *f = fopen(path, "r");
    if (NULL == f)
    {
        return;
    }
    char list[256] = "";
    const int read_ok = NULL != fgets(list, sizeof(list), f);
    fclose(f);
    for (char *p = list; read_ok && '\0' != *p && '\n' != *p;)
    {
        char *end = NULL;
        long lo = strtol(p, &end, 10);
        long hi = lo;
        if (end == p)
        {
            break;
        }
        if ('-' == *end)
        {
            p = end + 1;
            hi = strtol(p, &end, 10);
        }
        for (long c = lo; c <= hi && c < CPU_SETSIZE; c++)
        {
            if (c >= 0)
            {
                CPU_CLR((int)c, set);
            }
        }
        p = ',' == *end ? end + 1 : end;
    }
}

/* Where the housekeeping and local-I/O threads run: with the conductor, the
 * driver's housekeeping core, never on a core a busy-spinning agent owns.
 * RUSTERON_DPDK_HOUSEKEEPING_CPU when set; else the conductor's CPU when it is
 * pinned and does not busy-spin; else the EAL's main lcore CPU unless a
 * spinning agent is there; else the CPUs the creating thread may use (taken
 * at create; the calling thread's when `native` is NULL) minus the spinning
 * agents' cores, SMT siblings included. Which agents spin where is the
 * threading mode's: the sender's and the receiver's CPUs under DEDICATED, the
 * sender's (the network agent's) under SHARED_NETWORK, and the conductor's
 * when its idle strategy busy-spins; under SHARED and INVOKER the
 * conductor's, whose one thread (the shared agent, the application's) runs
 * everything, whatever the conductor's own strategy. The mode, the agents'
 * CPUs and the conductor's idle strategy come from the driver context once
 * installed, else from AERON_THREADING_MODE, AERON_*_CPU_AFFINITY and
 * AERON_CONDUCTOR_IDLE_STRATEGY. Returns 1 with `set` filled, 0 to leave the
 * thread unpinned, -1 when the env is bad. */
int rusteron_dpdk_housekeeping_affinity(const rusteron_dpdk_transport_t *native, cpu_set_t *set)
{
    CPU_ZERO(set);
    const int installed = NULL != native && native->agent_cpus_known;
    const int mode = installed ? native->threading_mode : rusteron_dpdk_threading_mode_env();
    const int conductor = installed ? native->agent_cpus[0] : rusteron_dpdk_agent_cpu("AERON_CONDUCTOR_CPU_AFFINITY");
    const int sender = installed ? native->agent_cpus[1] : rusteron_dpdk_agent_cpu("AERON_SENDER_CPU_AFFINITY");
    const int receiver = installed ? native->agent_cpus[2] : rusteron_dpdk_agent_cpu("AERON_RECEIVER_CPU_AFFINITY");
    const int conductor_spins =
        installed ? native->conductor_spins : rusteron_dpdk_idle_spins(getenv("AERON_CONDUCTOR_IDLE_STRATEGY"));
    int spinners[3] = { sender, receiver, conductor_spins ? conductor : -1 };
    if (RUSTERON_DPDK_THREADING_SHARED_NETWORK == mode)
    {
        spinners[1] = -1; /* no receiver agent: the network agent is on the sender's CPU */
    }
    else if (RUSTERON_DPDK_THREADING_SHARED == mode || RUSTERON_DPDK_THREADING_INVOKER == mode)
    {
        spinners[0] = -1;
        spinners[1] = -1;
        spinners[2] = conductor;
    }
    const size_t spinner_count = sizeof(spinners) / sizeof(spinners[0]);

    /* Every CPU but the spinning agents' cores (SMT siblings included): a
     * busy-spinning agent owns its whole core. */
    cpu_set_t free_cpus;
    CPU_ZERO(&free_cpus);
    for (int cpu = 0; cpu < CPU_SETSIZE; cpu++)
    {
        CPU_SET(cpu, &free_cpus);
    }
    for (size_t i = 0; i < spinner_count; i++)
    {
        rusteron_dpdk_clear_core(&free_cpus, spinners[i]);
    }

    /* The candidates: what this thread may run on. */
    if (NULL != native && native->process_cpus_known)
    {
        for (int cpu = 0; cpu < CPU_SETSIZE && cpu < 1024; cpu++)
        {
            if (native->process_cpus[cpu / 64] & (UINT64_C(1) << (cpu % 64)))
            {
                CPU_SET(cpu, set);
            }
        }
    }
    else if (0 != sched_getaffinity(0, sizeof(*set), set))
    {
        CPU_ZERO(set);
    }

    const char *env = getenv("RUSTERON_DPDK_HOUSEKEEPING_CPU");
    if (NULL != env && '\0' != env[0])
    {
        char *end = NULL;
        long v = strtol(env, &end, 10);
        if ('\0' != *end || v < 0 || v >= CPU_SETSIZE)
        {
            rusteron_dpdk_set_error("RUSTERON_DPDK_HOUSEKEEPING_CPU must be a CPU number");
            return -1;
        }
        const int cpu = (int)v;
        /* An override, not an exemption: a core a busy-spinning agent owns is
         * refused. Whether the thread can actually be pinned there is the
         * kernel's answer, and a pin that does not land fails the start (see
         * rusteron_dpdk_housekeeping_start): pthread_setaffinity_np is bounded
         * by the cpuset cgroup, not by the creating thread's own mask. */
        if (cpu < 0 || cpu >= CPU_SETSIZE || !CPU_ISSET(cpu, &free_cpus))
        {
            char detail[256];
            snprintf(detail, sizeof(detail),
                     "RUSTERON_DPDK_HOUSEKEEPING_CPU %d is on the core a busy-spinning agent owns (SMT siblings "
                     "included): give housekeeping a CPU of its own",
                     cpu);
            rusteron_dpdk_set_error(detail);
            return -1;
        }
        CPU_ZERO(set);
        CPU_SET(cpu, set);
        return 1;
    }

    /* The candidates, minus every spinning agent's core. */
    for (size_t i = 0; i < spinner_count; i++)
    {
        rusteron_dpdk_clear_core(set, spinners[i]);
    }

    /* The conductor's CPU, then the EAL main lcore's, when no spinner owns it
     * and this thread may run on it. */
    const int preferred[] = { conductor, rusteron_dpdk_eal_main_cpu() };
    for (size_t i = 0; i < sizeof(preferred) / sizeof(preferred[0]); i++)
    {
        if (preferred[i] >= 0 && preferred[i] < CPU_SETSIZE && CPU_ISSET(preferred[i], &free_cpus) &&
            CPU_ISSET(preferred[i], set))
        {
            CPU_ZERO(set);
            CPU_SET(preferred[i], set);
            return 1;
        }
    }
    return CPU_COUNT(set) > 0 ? 1 : 0;
}

int rusteron_dpdk_housekeeping_start(rusteron_dpdk_transport_t *native)
{
    cpu_set_t set;
    const int pin = rusteron_dpdk_housekeeping_affinity(native, &set);
    if (pin < 0)
    {
        return -1;
    }
    native->housekeeping_stop = 0;
    if (0 != pthread_create(&native->housekeeping_thread, NULL, rusteron_dpdk_housekeeping_main, native))
    {
        rusteron_dpdk_set_error("cannot start the housekeeping thread");
        return -1;
    }
    native->housekeeping_running = 1;
    pthread_setname_np(native->housekeeping_thread, "dpdk-housekeep");
    if (pin > 0)
    {
        const int why = pthread_setaffinity_np(native->housekeeping_thread, sizeof(set), &set);
        if (0 != why)
        {
            /* Left unpinned it would run wherever the scheduler puts it, on a
             * trading core included. Fail instead: the start is undone. */
            rusteron_dpdk_housekeeping_stop(native);
            char cpus[128];
            char detail[256];
            snprintf(detail, sizeof(detail), "cannot pin the housekeeping thread to CPU %s",
                     rusteron_dpdk_cpu_set_list(&set, cpus, sizeof(cpus)));
            rusteron_dpdk_set_error_code(detail, why);
            return -1;
        }
    }
    return 0;
}

void rusteron_dpdk_housekeeping_stop(rusteron_dpdk_transport_t *native)
{
    if (!native->housekeeping_running)
    {
        return;
    }
    __atomic_store_n(&native->housekeeping_stop, 1, __ATOMIC_RELEASE);
    pthread_join(native->housekeeping_thread, NULL);
    native->housekeeping_running = 0;
}

static void rusteron_dpdk_port_select_mac(rusteron_dpdk_port_t *p, const char *mac_text)
{
    /* Validated by the config check: empty, or a well-formed MAC. */
    p->has_mac = '\0' != mac_text[0] && 0 == rusteron_dpdk_parse_mac(mac_text, p->want_mac);
    snprintf(p->mac_text, sizeof(p->mac_text), "%s", p->has_mac ? mac_text : "");
}

int rusteron_dpdk_runtime_init(rusteron_dpdk_transport_t *native)
{
    cpu_set_t creator_cpus;
    if (0 == sched_getaffinity(0, sizeof(creator_cpus), &creator_cpus))
    {
        for (int cpu = 0; cpu < CPU_SETSIZE && cpu < 1024; cpu++)
        {
            if (CPU_ISSET(cpu, &creator_cpus))
            {
                native->process_cpus[cpu / 64] |= UINT64_C(1) << (cpu % 64);
            }
        }
        native->process_cpus_known = 1;
    }

    native->ops = rusteron_dpdk_active_ops();
    if (!native->local_io_running)
    {
        native->local_epfd = -1;
    }
    if (!native->housekeeping_lock_ready)
    {
        pthread_mutex_init(&native->housekeeping_lock, NULL);
        pthread_mutex_init(&native->port_table_lock, NULL);
        native->housekeeping_lock_ready = 1;
    }

    native->sender.role = RUSTERON_DPDK_ROLE_SENDER;
    memcpy(native->sender.pci, native->config.sender_pci, sizeof(native->sender.pci));
    rusteron_dpdk_port_select_mac(&native->sender, native->config.sender_mac);
    memcpy(native->sender.local_ipv4, native->config.sender_ipv4, sizeof(native->sender.local_ipv4));
    native->sender.prefix_len = native->config.sender_prefix_len;
    memcpy(native->sender.gateway_ipv4, native->config.sender_gateway, sizeof(native->sender.gateway_ipv4));

    native->receiver.role = RUSTERON_DPDK_ROLE_RECEIVER;
    memcpy(native->receiver.pci, native->config.receiver_pci, sizeof(native->receiver.pci));
    rusteron_dpdk_port_select_mac(&native->receiver, native->config.receiver_mac);
    memcpy(native->receiver.local_ipv4, native->config.receiver_ipv4, sizeof(native->receiver.local_ipv4));
    native->receiver.prefix_len = native->config.receiver_prefix_len;
    memcpy(native->receiver.gateway_ipv4, native->config.receiver_gateway, sizeof(native->receiver.gateway_ipv4));

    if (rusteron_dpdk_parse_ipv4(native->config.sender_ipv4, &native->sender.local_ip) < 0 ||
        rusteron_dpdk_parse_ipv4(native->config.sender_gateway, &native->sender.gateway_ip) < 0)
    {
        rusteron_dpdk_set_error("sender IPv4 address or gateway is not a valid dotted quad");
        return -1;
    }
    if (rusteron_dpdk_parse_ipv4(native->config.receiver_ipv4, &native->receiver.local_ip) < 0 ||
        rusteron_dpdk_parse_ipv4(native->config.receiver_gateway, &native->receiver.gateway_ip) < 0)
    {
        rusteron_dpdk_set_error("receiver IPv4 address or gateway is not a valid dotted quad");
        return -1;
    }

    if (rusteron_dpdk_runtime_net_init(native) < 0)
    {
        return -1;
    }

    if (rusteron_dpdk_runtime_eal_up(native) < 0)
    {
        return -1;
    }

    /* The clock, as the config chooses (Rust parses RUSTERON_DPDK_CLOCK):
     * started once the EAL is up, so rte_get_tsc_hz() holds its rate, and
     * only once the EAL guard has passed, so a second transport refused while
     * one runs never touches the clock its agents read. Before the ports and
     * the threads below, which read it; its start-up measurement of the
     * counter (25 ms) spins here, before any of them run. */
    rusteron_dpdk_clock_start(native->config.clock);

    if (rusteron_dpdk_init_port(native, &native->sender) < 0)
    {
        rusteron_dpdk_runtime_cleanup(native);
        return -1;
    }
    if (rusteron_dpdk_init_port(native, &native->receiver) < 0)
    {
        rusteron_dpdk_runtime_cleanup(native);
        return -1;
    }
    if (0 == native->rx_max_bursts)
    {
        const uint32_t ring = native->sender.rx_descriptors < native->receiver.rx_descriptors
            ? native->sender.rx_descriptors
            : native->receiver.rx_descriptors;
        const uint32_t bursts = ring / native->config.burst_size; /* burst_size validated 1..=256 */
        native->rx_max_bursts = bursts > 8 ? 8 : bursts < 1 ? 1 : bursts;
    }

    /* The lab event log (RUSTERON_DPDK_EVENT_LOG): its rings exist before
     * any thread that writes them runs. */
    if (rusteron_dpdk_events_open(native) < 0)
    {
        rusteron_dpdk_runtime_cleanup(native);
        return -1;
    }

    if (rusteron_dpdk_housekeeping_enabled && rusteron_dpdk_housekeeping_start(native) < 0)
    {
        rusteron_dpdk_runtime_cleanup(native);
        return -1;
    }

    cpu_set_t local_io_cpus;
    const int local_io_pin = rusteron_dpdk_housekeeping_affinity(native, &local_io_cpus);
    if (local_io_pin < 0 ||
        rusteron_dpdk_local_io_start(native, local_io_pin > 0 ? &local_io_cpus : NULL) < 0)
    {
        rusteron_dpdk_runtime_cleanup(native);
        return -1;
    }

    return 0;
}

/* Tear one role port down in reverse of its init order: stop, close, free pool.
 * A port configured but without both queues set up is left open: DPDK's ENA
 * close releases every configured queue without checking it was set up
 * (ena_tx_queue_release, DPDK 24.11) and would crash the process; the process
 * is failing to start anyway. */
static void rusteron_dpdk_port_teardown(rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *p)
{
    const rusteron_dpdk_port_ops_t *ops = native->ops;
    if (p->reset_watched)
    {
        ops->unwatch_reset(p->port_id, &p->nic_resets);
        p->reset_watched = 0;
    }
    if (p->lsc_watched)
    {
        ops->unwatch_lsc(p->port_id, &p->nic_link_changes);
        p->lsc_watched = 0;
    }
    if (p->started)
    {
        ops->dev_stop(p->port_id);
    }
    if (p->configured && p->queues_ready)
    {
        ops->dev_close(p->port_id);
    }
    p->started = 0;
    p->configured = 0;
    p->queues_ready = 0;
    /* frames still queued are the mempool's: back before it goes */
    rusteron_dpdk_port_tx_drop_queued(native, p);
    if (NULL != p->mempool)
    {
        ops->mempool_free(p->mempool);
        p->mempool = NULL;
    }
}

/* Reverse of initialization order: the receiver is initialized second, so it is
 * torn down first, then the sender. */
void rusteron_dpdk_runtime_cleanup(rusteron_dpdk_transport_t *native)
{
    /* The housekeeping thread reads the counters and the ports: stop it first. */
    rusteron_dpdk_housekeeping_stop(native);
    rusteron_dpdk_local_io_stop(native);
    /* What the agents logged since housekeeping's last pass, then the file. */
    rusteron_dpdk_events_close(native);
    free(native->host_ips);
    native->host_ips = NULL;
    native->host_ip_count = 0;

    /* The counters went back to the driver's counters manager when each port's
     * last transport closed (close_func, on the conductor, while the manager
     * was alive). By now the driver may be gone with its manager and CnC, so
     * only local state is released here, never the manager touched. */
    rusteron_dpdk_counters_forget(&native->receiver.counters);
    rusteron_dpdk_counters_forget(&native->sender.counters);

    rusteron_dpdk_port_teardown(native, &native->receiver);
    rusteron_dpdk_port_teardown(native, &native->sender);
    native->eal_up = 0;
}

int rusteron_dpdk_runtime_probe_device(
    rusteron_dpdk_transport_t *native, const char *pci_bdf, uint16_t *port_id)
{
    const rusteron_dpdk_config_t *cfg = &native->config;
    char message[512];

    if (NULL == pci_bdf || NULL == port_id)
    {
        rusteron_dpdk_set_error("probe_device requires non-NULL bdf and port_id");
        return -1;
    }

    int configured = (0 == strcmp(pci_bdf, cfg->sender_pci)) ||
                     (0 == strcmp(pci_bdf, cfg->receiver_pci));
    if (!configured)
    {
        snprintf(message, sizeof(message),
                 "device %s is not a configured ENA (only %s and %s are allow-listed)",
                 pci_bdf, cfg->sender_pci, cfg->receiver_pci);
        rusteron_dpdk_set_error(message);
        return -1;
    }

    if (native->ops->probe_port(pci_bdf, NULL, port_id) < 0)
    {
        snprintf(message, sizeof(message),
                 "device %s is allow-listed but not probed by DPDK", pci_bdf);
        rusteron_dpdk_set_error(message);
        return -1;
    }

    return 0;
}

void rusteron_dpdk_transport_test_dump(
    const rusteron_dpdk_transport_t *native,
    uint16_t *sender_port, uintptr_t *sender_pool,
    uint16_t *receiver_port, uintptr_t *receiver_pool)
{
    if (NULL != sender_port)
    {
        *sender_port = native->sender.port_id;
    }
    if (NULL != sender_pool)
    {
        *sender_pool = (uintptr_t)native->sender.mempool;
    }
    if (NULL != receiver_port)
    {
        *receiver_port = native->receiver.port_id;
    }
    if (NULL != receiver_pool)
    {
        *receiver_pool = (uintptr_t)native->receiver.mempool;
    }
}

/* Test-only ARP hooks (see internal.h). */
int rusteron_dpdk_transport_test_arp_seed(
    rusteron_dpdk_transport_t *native, const char *ip_str, const uint8_t mac[6])
{
    if (NULL == native || NULL == ip_str || NULL == mac)
    {
        rusteron_dpdk_set_error("arp_seed requires non-NULL transport, ip and mac");
        return -1;
    }

    uint32_t ip;
    if (rusteron_dpdk_parse_ipv4(ip_str, &ip) < 0)
    {
        rusteron_dpdk_set_error("arp_seed ip must be a valid dotted quad");
        return -1;
    }

    /* Each NIC keeps its own table; a test peer is reachable from both. */
    rusteron_dpdk_port_t *ports[2] = { &native->sender, &native->receiver };
    for (int p = 0; p < 2; p++)
    {
        if (rusteron_dpdk_arp_seed(&ports[p]->arp, ip, mac, rusteron_dpdk_clock_ns()) < 0)
        {
            rusteron_dpdk_set_error("arp_seed: ARP table is full");
            return -1;
        }
    }
    return 0;
}

int rusteron_dpdk_transport_test_arp_rx(
    rusteron_dpdk_transport_t *native, int role, const uint8_t *frame, size_t frame_len)
{
    if (NULL == native)
    {
        rusteron_dpdk_set_error("arp_rx requires a non-NULL transport");
        return -1;
    }
    rusteron_dpdk_port_t *port =
        (RUSTERON_DPDK_ROLE_RECEIVER == role) ? &native->receiver : &native->sender;
    return rusteron_dpdk_arp_handle_frame(&port->arp, native, port, frame, frame_len);
}

void rusteron_dpdk_transport_test_rx_stats(
    const rusteron_dpdk_transport_t *native, rusteron_dpdk_rx_stats_t *out)
{
    if (NULL == native || NULL == out)
    {
        return;
    }
    /* Each NIC counts its own; the test view is their sum. */
    const uint64_t *a = (const uint64_t *)&native->sender.rx_stats;
    const uint64_t *b = (const uint64_t *)&native->receiver.rx_stats;
    uint64_t *o = (uint64_t *)out;
    for (size_t i = 0; i < sizeof(*out) / sizeof(uint64_t); i++)
    {
        o[i] = a[i] + b[i];
    }
}

/* Nothing else of the transport's may share a busy-spinning network agent's
 * core (SMT siblings included). Under DEDICATED those are the sender's and
 * the receiver's; under SHARED_NETWORK, SHARED and INVOKER the one thread
 * with all of the network work. The EAL's own threads (interrupts, alarms),
 * confined to its main lcore's CPU, must be off every one of them, in every
 * mode. In the three shared modes so must the housekeeping and local-I/O
 * threads, which rusteron_dpdk_housekeeping_affinity found no CPU off that
 * core when `pin` is 0, as when the thread creating the transport may run
 * only there (an application that pinned the thread it runs an INVOKER
 * driver on before starting it). Either refuses the install (-1), saying how
 * to give them another; an agent without a known CPU (an install the Rust
 * side did not validate) is left as it is. */
static int rusteron_dpdk_agent_core_check(const rusteron_dpdk_transport_t *native, int pin)
{
    const char *mode;
    const char *names[2] = { "network agent", NULL };
    int spinners[2] = { -1, -1 };
    switch (native->threading_mode)
    {
        case RUSTERON_DPDK_THREADING_DEDICATED:
            mode = "DEDICATED";
            names[0] = "sender";
            names[1] = "receiver";
            spinners[0] = native->agent_cpus[1];
            spinners[1] = native->agent_cpus[2];
            break;
        case RUSTERON_DPDK_THREADING_SHARED_NETWORK:
            mode = "SHARED_NETWORK";
            spinners[0] = native->agent_cpus[1];
            break;
        case RUSTERON_DPDK_THREADING_SHARED:
            mode = "SHARED";
            spinners[0] = native->agent_cpus[0];
            break;
        case RUSTERON_DPDK_THREADING_INVOKER:
            mode = "INVOKER";
            spinners[0] = native->agent_cpus[0];
            break;
        default:
            return 0;
    }
    char message[512];
    const int main_cpu = rusteron_dpdk_eal_main_cpu();
    for (int i = 0; i < 2; i++)
    {
        const int spinner = spinners[i];
        if (spinner < 0 || spinner >= CPU_SETSIZE)
        {
            continue;
        }
        cpu_set_t off_core;
        CPU_ZERO(&off_core);
        for (int cpu = 0; cpu < CPU_SETSIZE; cpu++)
        {
            CPU_SET(cpu, &off_core);
        }
        rusteron_dpdk_clear_core(&off_core, spinner);
        if (main_cpu >= 0 && main_cpu < CPU_SETSIZE && !CPU_ISSET(main_cpu, &off_core))
        {
            snprintf(message, sizeof(message),
                     "AERON_THREADING_MODE %s: the EAL's main lcore (CPU %d) is on the core the %s busy-spins on "
                     "(CPU %d), and the EAL's own threads (interrupts, alarms) run there: create the transport "
                     "from a thread confined to other CPUs, or give the main lcore another with "
                     "RUSTERON_DPDK_EAL_ARGS (-l <cpu>)",
                     mode, main_cpu, names[i], spinner);
            rusteron_dpdk_set_error(message);
            return -1;
        }
        if (0 == pin && RUSTERON_DPDK_THREADING_DEDICATED != native->threading_mode)
        {
            snprintf(message, sizeof(message),
                     "AERON_THREADING_MODE %s: the housekeeping and local-I/O threads have no CPU off the core the "
                     "network agent busy-spins on (CPU %d), the only one the thread creating the transport may run "
                     "on: create it from a thread allowed others (an INVOKER application pins its thread once the "
                     "driver has started, as Aeron does then), or set RUSTERON_DPDK_HOUSEKEEPING_CPU",
                     mode, spinner);
            rusteron_dpdk_set_error(message);
            return -1;
        }
    }
    return 0;
}

/* The driver context is known (install): take its threading mode and its
 * agents' CPUs, however they were set, and move the housekeeping and
 * local-I/O threads off the spinning ones. */
int rusteron_dpdk_runtime_agents_known(
    rusteron_dpdk_transport_t *native, int threading_mode, int conductor, int sender, int receiver,
    const char *conductor_idle)
{
    if (RUSTERON_DPDK_THREADING_DEDICATED != threading_mode)
    {
        /* One thread sends and receives: a pacing wait would stall its
         * receive half, so none, and an operator's non-zero one is refused. */
        if (native->tx_pace_spin_set && 0 != native->tx_pace_spin_ns)
        {
            rusteron_dpdk_set_error(
                "RUSTERON_DPDK_TX_PACE_SPIN_US must be 0 or unset under AERON_THREADING_MODE SHARED_NETWORK, "
                "SHARED or INVOKER: a send waiting out a pacing debt would stall the receive half of its agent");
            return -1;
        }
        native->tx_pace_spin_ns = 0;
        /* Re-offering a refused tail spins to a deadline the same way. */
        if (0 != native->tx_retry_ns)
        {
            rusteron_dpdk_set_error(
                "RUSTERON_DPDK_TX_RETRY_US must be 0 or unset under AERON_THREADING_MODE SHARED_NETWORK, "
                "SHARED or INVOKER: a send spinning to re-offer a refused tail would stall the receive half of "
                "its agent");
            return -1;
        }
    }
    native->threading_mode = threading_mode;
    /* The context may still name its `backoff` default when the environment
     * chose the strategy (ffi.rs effective_idle_strategy): either counts. */
    native->conductor_spins =
        rusteron_dpdk_idle_spins(conductor_idle) || rusteron_dpdk_idle_spins(getenv("AERON_CONDUCTOR_IDLE_STRATEGY"));
    native->agent_cpus[0] = conductor;
    native->agent_cpus[1] = sender;
    native->agent_cpus[2] = receiver;
    native->agent_cpus_known = 1;
    cpu_set_t set;
    const int pin = rusteron_dpdk_housekeeping_affinity(native, &set);
    if (rusteron_dpdk_agent_core_check(native, pin) < 0)
    {
        return -1;
    }
    if (pin <= 0)
    {
        return 0;
    }
    /* A pin that does not land leaves the thread free to run on a trading
     * core. Fail the install rather than report an isolation that is not
     * there. */
    char cpus[128];
    char detail[256];
    const char *on = rusteron_dpdk_cpu_set_list(&set, cpus, sizeof(cpus));
    if (native->housekeeping_running)
    {
        const int why = pthread_setaffinity_np(native->housekeeping_thread, sizeof(set), &set);
        if (0 != why)
        {
            snprintf(detail, sizeof(detail), "cannot pin the housekeeping thread to CPU %s", on);
            rusteron_dpdk_set_error_code(detail, why);
            return -1;
        }
    }
    if (native->local_io_running)
    {
        const int why = pthread_setaffinity_np(native->local_io_thread, sizeof(set), &set);
        if (0 != why)
        {
            snprintf(detail, sizeof(detail), "cannot pin the local-I/O thread to CPU %s", on);
            rusteron_dpdk_set_error_code(detail, why);
            return -1;
        }
    }
    return 0;
}
