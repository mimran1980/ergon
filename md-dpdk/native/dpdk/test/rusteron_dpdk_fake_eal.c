/*
 * ergon-md-dpdk DPDK transport — fake EAL seam for tests.
 *
 * Provides the same rusteron_dpdk_eal_init / rusteron_dpdk_eal_is_initialized
 * symbols the production rusteron_dpdk_eal.c defines, so test binaries can link
 * the runtime without libdpdk. The singleton semantics mirror the real EAL: a
 * second init in the same process is rejected by the runtime's
 * ever-initialized guard before this seam is consulted.
 */
#include "rusteron_dpdk_fake.h"
#include "rusteron_dpdk_internal.h"

static int rusteron_dpdk_fake_eal_initialized = 0;
static int rusteron_dpdk_fake_eal_main_cpu = -1;
/* The DPDK the fake EAL reports (rusteron_dpdk_eal_version): 25.11 until set. */
static int rusteron_dpdk_fake_eal_year = 25;
static int rusteron_dpdk_fake_eal_month = 11;
/* The calling thread registered (as rte_thread_register gives it an lcore). */
static _Thread_local int rusteron_dpdk_fake_eal_registered = 0;

void rusteron_dpdk_fake_eal_reset(void)
{
    rusteron_dpdk_fake_eal_initialized = 0;
    rusteron_dpdk_fake_eal_main_cpu = -1;
    rusteron_dpdk_fake_eal_year = 25;
    rusteron_dpdk_fake_eal_month = 11;
}

void rusteron_dpdk_fake_set_eal_version(int year, int month)
{
    rusteron_dpdk_fake_eal_year = year;
    rusteron_dpdk_fake_eal_month = month;
}

void rusteron_dpdk_eal_version(int *year, int *month)
{
    *year = rusteron_dpdk_fake_eal_year;
    *month = rusteron_dpdk_fake_eal_month;
}

void rusteron_dpdk_fake_set_eal_main_cpu(int cpu)
{
    rusteron_dpdk_fake_eal_main_cpu = cpu;
}

int rusteron_dpdk_eal_is_initialized(void)
{
    return rusteron_dpdk_fake_eal_initialized ? 1 : 0;
}

int rusteron_dpdk_eal_init(const rusteron_dpdk_eal_params_t *params, char *errbuf, size_t errlen)
{
    (void)params;
    (void)errbuf;
    (void)errlen;
    rusteron_dpdk_fake_eal_initialized = 1;
    return 0;
}

/* Thread registration (plan §7.2): DPDK-free under the fake EAL, which only
 * notes which threads registered, for the fake port's timer service. */
int rusteron_dpdk_eal_thread_register(void)
{
    rusteron_dpdk_fake_eal_registered = 1;
    return 0;
}

int rusteron_dpdk_eal_thread_unregister(void)
{
    rusteron_dpdk_fake_eal_registered = 0;
    return 0;
}

int rusteron_dpdk_fake_eal_thread_registered(void)
{
    return rusteron_dpdk_fake_eal_registered;
}

int rusteron_dpdk_eal_main_cpu(void)
{
    return rusteron_dpdk_fake_eal_main_cpu; /* -1, no pinning, unless a test set one */
}
