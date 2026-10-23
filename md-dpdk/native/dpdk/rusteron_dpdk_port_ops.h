/*
 * ergon-md-dpdk DPDK transport — injectable port operations (plan §7.2).
 *
 * The EAL and port seams are deliberately split from the orchestration layer
 * (runtime.c) so the runtime can be linked and deterministically tested without
 * linking libdpdk:
 *
 *   - The default port-ops getter rusteron_dpdk_port_ops_real() is provided by
 *     rusteron_dpdk_port.c (real rte_eth_* calls) in production, and by
 *     test/rusteron_dpdk_fake_port.c in test builds. The linker picks the
 *     definition from whichever archive is linked, exactly one of the two.
 *
 * No DPDK type leaks through this header: handles are opaque pointers and
 * offload/queue values are plain integers so the fake never needs rte headers.
 */
#ifndef RUSTERON_DPDK_PORT_OPS_H
#define RUSTERON_DPDK_PORT_OPS_H

#include <stddef.h>
#include <stdint.h>

#include "rusteron_dpdk_packet.h" /* rusteron_dpdk_mbuf_t, RUSTERON_DPDK_ETH_ADDR_LEN */

#ifdef __cplusplus
extern "C" {
#endif

/* Fixed record length for PMD extended-stat names (rte_eth_xstat_name is that
 * size in DPDK); the fake packs the same stride so the two stay interchangeable
 * (plan §9). */
#define RUSTERON_DPDK_XSTAT_NAME_LEN 64

/* How an ENA's low-latency queue (LLQ) BAR, BAR 2, is mapped in the process
 * (dev_info's ena_llq). DPDK maps an ENA's prefetchable BAR through sysfs
 * (igb_uio, uio_pci_generic) as `<bdf>/resource2_wc`, write-combined, when it
 * can, else as the uncached `resource2`; without write-combining every send
 * pushes its descriptors and headers through uncached stores, about 100 times
 * slower. vfio-pci maps it through its device file, which /proc/self/maps
 * cannot attribute to the BAR, let alone show write-combined. */
#define RUSTERON_DPDK_ENA_LLQ_NA 0       /* not an ENA (not checked) */
#define RUSTERON_DPDK_ENA_LLQ_WC 1       /* resource2_wc mapped: write-combined */
#define RUSTERON_DPDK_ENA_LLQ_UNCACHED 2 /* resource2 mapped, not resource2_wc */
#define RUSTERON_DPDK_ENA_LLQ_UNSEEN 3   /* no sysfs mapping of BAR 2 (vfio-pci, or none) */
#define RUSTERON_DPDK_ENA_LLQ_UNKNOWN 4  /* /proc/self/maps unreadable */

typedef struct rusteron_dpdk_port_ops_stct
{
    /* Resolve a PCI BDF (and, when `mac` is non-NULL, the port on it with that
     * MAC: Azure MANA) to a DPDK port id. -1 if no such port is probed. */
    int (*probe_port)(const char *pci_bdf, const uint8_t *mac, uint16_t *port_id);

    /*
     * Read the device MAC, driver name, whether IPv4/UDP checksum offloads,
     * RX hardware timestamps and TX MBUF_FAST_FREE are supported, how an
     * ENA's LLQ BAR is mapped (RUSTERON_DPDK_ENA_LLQ_*; NA for any other
     * driver), and the descriptor counts a queue may have: desc_lim is RX
     * minimum, RX maximum, TX minimum, TX maximum (0: no limit reported). 0 on
     * success.
     */
    int (*dev_info)(
        uint16_t port_id,
        const char *pci_bdf,
        uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN],
        char *driver_name, size_t driver_name_len,
        int *csum_offload_ok,
        int *ena_llq,
        int *rx_timestamp_ok,
        int *tx_fast_free_ok,
        uint16_t *max_mtu,
        uint16_t desc_lim[4]);

    /* Configure one RX and one TX queue plus the requested offloads. With
     * RUSTERON_DPDK_RX_OFFLOAD_TIMESTAMP, rx_burst fills hw_rx_ns. `lsc`
     * asks for link-status-change interrupts (only where lsc_capable). */
    int (*dev_configure)(
        uint16_t port_id, uint16_t rx_rings, uint16_t tx_rings,
        uint64_t rx_offloads, uint64_t tx_offloads, int lsc);

    /* Whether the port can raise link-status-change interrupts
     * (RTE_ETH_DEV_INTR_LSC in its dev_flags): a PMD refuses a configuration
     * that asks for them otherwise. 1 or 0. */
    int (*lsc_capable)(uint16_t port_id);

    /* NUMA node the PMD reports for the port, or -1 when it reports none.
     * The runtime places that port's pool and queues there, and refuses a
     * pair whose two ports report different nodes. */
    int (*socket_id)(uint16_t port_id);

    /* Create a packet mbuf pool on `socket_id` (-1: no NUMA node); returns an
     * opaque handle (NULL on failure). */
    /* Each mbuf holds `data_room` bytes after the headroom, so a frame of the
     * configured MTU always fits one segment. */
    void *(*mempool_create)(const char *name, uint32_t n, uint16_t cache_size, uint16_t data_room, int socket_id);
    /* The bytes of frame one of the pool's mbufs holds (its data room after
     * the headroom), read from the pool: no mbuf is taken, as one taken and
     * returned by the creating thread would leave a cache's worth in that
     * lcore's mempool cache, out of the agents' reach. */
    uint32_t (*mbuf_room)(void *mempool);

    /* Queue 0 on each direction, backed by the role mempool for RX.
     * `socket_id` is the port's NUMA node, or -1 when it has none. Each queue
     * starts from the PMD's default queue configuration (dev_info's
     * default_rxconf / default_txconf); `free_thresh` non-zero replaces its
     * rx_free_thresh / tx_free_thresh, 0 keeps the PMD's. */
    int (*rx_queue_setup)(
        uint16_t port_id, uint16_t queue_id, uint16_t nb_desc, void *mempool, int socket_id, uint16_t free_thresh);
    int (*tx_queue_setup)(uint16_t port_id, uint16_t queue_id, uint16_t nb_desc, int socket_id, uint16_t free_thresh);

    /* Apply the L3 MTU (payload + IPv4 + UDP). */
    int (*dev_set_mtu)(uint16_t port_id, uint16_t mtu);

    int (*dev_start)(uint16_t port_id);

    /* Poll the link until UP or `timeout_ms` elapses. 0 when UP. */
    int (*link_wait_ready)(uint16_t port_id, uint32_t timeout_ms);

    int (*dev_stop)(uint16_t port_id);
    int (*dev_close)(uint16_t port_id);
    void (*mempool_free)(void *mempool);

    /* Data path (plan §7.4). */

    /* Allocate one packet mbuf from the pool; fills the view's opaque/data/
     * capacity. 0 on success, -1 when the pool is exhausted. */
    int (*mbuf_alloc)(void *mempool, rusteron_dpdk_mbuf_t *m);

    /* Return a view's mbuf to the pool: views never sent (a build failure
     * before batching, or tx_burst's rejected tail). */
    void (*mbuf_release)(rusteron_dpdk_mbuf_t *m);

    /*
     * Transmit a burst. The accepted prefix becomes the NIC's; the rejected
     * tail stays the caller's, to offer again or release with mbuf_release
     * (rusteron_dpdk_port_tx does both). Returns the number of frames
     * accepted (0..=nb, a contiguous prefix from index 0).
     */
    uint16_t (*tx_burst)(
        uint16_t port_id, uint16_t tx_queue_id,
        rusteron_dpdk_mbuf_t **pkts, uint16_t nb);

    /*
     * Receive a burst. The impl fills every returned view (opaque/data/
     * capacity/frame_len/nb_segs/rx_ol_flags); the caller owns the views and
     * must release them via mbuf_release once processed. Returns the number of
     * frames received (0..=nb, a contiguous prefix from index 0).
     */
    uint16_t (*rx_burst)(
        uint16_t port_id, uint16_t rx_queue_id,
        rusteron_dpdk_mbuf_t **pkts, uint16_t nb);

    /* Extended statistics (plan §9). xstats_count returns the number of PMD
     * extended stats; xstats_names packs that many NUL-terminated names into
     * `names`, each at a fixed RUSTERON_DPDK_XSTAT_NAME_LEN stride (the PMD
     * names come from rte_eth_xstat_name, which is that size); xstats_get
     * fills `values[0..count)` with the current value of the first `count`
     * xstats in id order. The _names/_get functions return 0 on success.
     * mempool_avail returns the number of free objects in a pool. */
    uint32_t (*xstats_count)(uint16_t port_id);
    int (*xstats_names)(uint16_t port_id, char *names, uint32_t count);
    int (*xstats_get)(uint16_t port_id, uint64_t *values, uint32_t count);
    /* The xstats with the given ids (indexes in xstats_names' order) into
     * `values[0..count)`, without reading the others: on ENA only the ids in
     * its ENI, SRD and customer-metric ranges cost an admin command. 0 on
     * success. */
    int (*xstats_get_by_id)(uint16_t port_id, const uint64_t *ids, uint64_t *values, uint32_t count);
    uint32_t (*mempool_avail)(void *mempool);
    /* The NIC's basic receive drop counters (rte_eth_stats: ierrors, imissed,
     * rx_nombuf), read by the housekeeping thread. 0 on success. */
    int (*basic_stats)(uint16_t port_id, uint64_t *ierrors, uint64_t *imissed, uint64_t *rx_nombuf);

    /* Health (housekeeping thread only, never an agent). Run the PMDs' DPDK
     * timers that are due: ENA's watchdog (device keep-alive, admin queue,
     * missing transmit completions) is one, and runs only through this. The
     * calling thread must be registered with DPDK. 0 on success. */
    int (*timer_service)(void);
    /* Count the port's device-reset requests (RTE_ETH_EVENT_INTR_RESET) into
     * *resets, atomically, from whichever thread the PMD raises them on;
     * unwatch_reset stops it before the port closes. 0 on success. */
    int (*watch_reset)(uint16_t port_id, uint64_t *resets);
    void (*unwatch_reset)(uint16_t port_id, uint64_t *resets);
    /* Count the port's link-status changes (RTE_ETH_EVENT_INTR_LSC) into
     * *changes, atomically, as watch_reset does; unwatch_lsc stops it before
     * the port closes. 0 on success. */
    int (*watch_lsc)(uint16_t port_id, uint64_t *changes);
    void (*unwatch_lsc)(uint16_t port_id, uint64_t *changes);

    /* Reclaim up to `budget` sent descriptors of the TX queue whose
     * completions the NIC has posted (rte_eth_tx_done_cleanup; 0: all), as
     * the PMD otherwise does inside a later tx_burst. Only the queue's own
     * thread (the agent owning the port) may call it. The number reclaimed,
     * or a negative errno: -ENOTSUP when the PMD lacks the op. */
    int (*tx_done_cleanup)(uint16_t port_id, uint16_t queue_id, uint32_t budget);
} rusteron_dpdk_port_ops_t;

/*
 * The active port operations. Production links rusteron_dpdk_port.a (real);
 * test binaries link rusteron_dpdk_fake.a instead. Exactly one of the two
 * archives must define this symbol in any given link.
 */
rusteron_dpdk_port_ops_t *rusteron_dpdk_port_ops_real(void);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_PORT_OPS_H */
