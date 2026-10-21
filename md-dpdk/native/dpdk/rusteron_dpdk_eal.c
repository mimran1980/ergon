/*
 * ergon-md-dpdk DPDK transport — real EAL seam (plan §7.2).
 *
 * Wraps rte_eal_init and tracks whether the EAL runtime is up (DPDK >= 23.11
 * exposes no public rte_eal_is_initialized). The argv is built from the typed
 * config: exactly two allow-listed devices (the sender and receiver) — a PCI
 * BDF via `-a <bdf>` for ENA, or a virtual-device name via `--vdev=<name>` for
 * the test/TAP path (plan §11.2) — plus a per-process file prefix,
 * primary-process mode, no telemetry, and either a hugepage directory
 * (production, so startup fails if hugetlbfs is unusable) or --no-huge (tests
 * without hugetlbfs).
 *
 * Only this translation unit and rusteron_dpdk_port.c reference libdpdk, so
 * the seam keeps every other translation unit DPDK-free and testable.
 */
/* pthread_{get,set}affinity_np and cpu_set_t; before any header. */
#define _GNU_SOURCE
#include "rusteron_dpdk_internal.h"

#include <dirent.h>
#include <pthread.h>
#include <sched.h>
#include <stdlib.h>

#include <rte_eal.h>
#include <rte_lcore.h>
#include <rte_version.h>

#include <stdio.h>
#include <string.h>

#define RUSTERON_DPDK_EAL_MAX_TIDS 1024

/* The process's thread ids (/proc/self/task), up to `cap`. */
static size_t rusteron_dpdk_eal_tids(pid_t *tids, size_t cap)
{
    size_t n = 0;
    DIR *dir = opendir("/proc/self/task");
    if (NULL == dir)
    {
        return 0;
    }
    for (struct dirent *e = readdir(dir); NULL != e && n < cap; e = readdir(dir))
    {
        if (e->d_name[0] >= '0' && e->d_name[0] <= '9')
        {
            tids[n++] = (pid_t)atoi(e->d_name);
        }
    }
    closedir(dir);
    return n;
}

/* The EAL's own threads (interrupts and alarms, multi-process messages) may
 * run on any CPU the caller could, less the lcores: the media driver's agent
 * and application CPUs among them, where one waking would preempt a
 * busy-spinning agent. Confine every thread rte_eal_init started (those not
 * in `before`) to the main lcore's CPUs, where housekeeping runs. */
static void rusteron_dpdk_eal_confine_new_threads(const pid_t *before, size_t nbefore)
{
    static pid_t after[RUSTERON_DPDK_EAL_MAX_TIDS];
    const size_t nafter = rusteron_dpdk_eal_tids(after, RUSTERON_DPDK_EAL_MAX_TIDS);
    rte_cpuset_t main_cpus = rte_lcore_cpuset(rte_get_main_lcore());
    for (size_t i = 0; i < nafter; i++)
    {
        int existed = 0;
        for (size_t j = 0; j < nbefore && !existed; j++)
        {
            existed = after[i] == before[j];
        }
        if (!existed)
        {
            (void)sched_setaffinity(after[i], sizeof(main_cpus), &main_cpus);
        }
    }
}

/* DPDK >= 23.11 exposes no public rte_eal_is_initialized(); the runtime owns
 * the only EAL instance in the process, so track initialization here. */
static int rusteron_dpdk_eal_initialized = 0;

int rusteron_dpdk_eal_is_initialized(void)
{
    return rusteron_dpdk_eal_initialized ? 1 : 0;
}

int rusteron_dpdk_eal_init(const rusteron_dpdk_eal_params_t *params, char *errbuf, size_t errlen)
{
    const rusteron_dpdk_config_t *cfg = params->config;
    const char *selectors[2];
    size_t nselectors = 0;
    selectors[nselectors++] = cfg->sender_pci;
    selectors[nselectors++] = cfg->receiver_pci;

    const char *mem_option;
    if (RUSTERON_DPDK_EAL_NO_HUGE == params->mode)
    {
        mem_option = "--no-huge";
    }
    else
    {
        mem_option = "--huge-dir";
    }

    /* argv[0] is the program name; EAL scans argv[1..] for its options. The
     * device arguments (`-a <bdf>` for an ENA, `-a <bdf>,mac=..` for MANA
     * vports, `--vdev=<name>` for a test vdev) come from the DPDK-free builder
     * the tests assert, split on its single spaces. */
    char allow[1024];
    if (rusteron_dpdk_eal_allow_args(cfg, allow, sizeof(allow)) < 0)
    {
        snprintf(errbuf, errlen, "cannot build the EAL device arguments for %s,%s", selectors[0], selectors[1]);
        return -1;
    }
    char *argv[64];
    int argc = 0;
    argv[argc++] = (char *)"rusteron-dpdk";
    char *save = NULL;
    for (char *tok = strtok_r(allow, " ", &save); NULL != tok && argc < 52; tok = strtok_r(NULL, " ", &save))
    {
        argv[argc++] = tok;
    }
    argv[argc++] = (char *)"--file-prefix";
    argv[argc++] = (char *)cfg->file_prefix;
    argv[argc++] = (char *)"--proc-type";
    argv[argc++] = (char *)"primary";
    argv[argc++] = (char *)"--no-telemetry";
    argv[argc++] = (char *)mem_option;
    if (RUSTERON_DPDK_EAL_NO_HUGE != params->mode)
    {
        argv[argc++] = (char *)cfg->hugepage_dir;
    }

    char version[64] = "";
    const char *ver = rte_version();
    if (NULL != ver)
    {
        snprintf(version, sizeof(version), "%s", ver);
    }

    /* rte_eal_init pins the calling thread to the main lcore (CPU 0 by
     * default). Every thread that caller later starts would inherit that pin:
     * the media driver's sender and receiver, for one, would share a core.
     * Restore the caller's own affinity once the EAL is up. */
    cpu_set_t caller_cpus;
    int have_caller_cpus = 0 == pthread_getaffinity_np(pthread_self(), sizeof(caller_cpus), &caller_cpus);
    static pid_t before[RUSTERON_DPDK_EAL_MAX_TIDS];
    const size_t nbefore = rusteron_dpdk_eal_tids(before, RUSTERON_DPDK_EAL_MAX_TIDS);

    int rc = rte_eal_init(argc, argv);
    if (rc >= 0 && have_caller_cpus)
    {
        (void)pthread_setaffinity_np(pthread_self(), sizeof(caller_cpus), &caller_cpus);
    }
    if (rc >= 0)
    {
        rusteron_dpdk_eal_confine_new_threads(before, nbefore);
    }
    if (rc < 0)
    {
        snprintf(errbuf, errlen,
                 "rte_eal_init failed (rc=%d, DPDK %s): cannot initialize EAL "
                 "with devices %s,%s and hugepages %s; verify the device binding (vfio-pci for an ENA, "
                 "the interfaces down for MANA) and hugetlbfs",
                 rc, version[0] != '\0' ? version : "<unknown>",
                 selectors[0], selectors[1],
                 RUSTERON_DPDK_EAL_NO_HUGE == params->mode ? "disabled" : cfg->hugepage_dir);
        return -1;
    }

    rusteron_dpdk_eal_initialized = 1;
    return 0;
}

void rusteron_dpdk_eal_version(int *year, int *month)
{
    /* The DPDK built against: a shared build runs one of the same ABI, and
     * ENA's device arguments changed at 24.11, an ABI boundary. */
    *year = RTE_VER_YEAR;
    *month = RTE_VER_MONTH;
}

int rusteron_dpdk_eal_thread_register(void)
{
    /* Only lcore-less threads need registration. A registered non-EAL thread
     * (and the EAL main lcore) already has a valid lcore id, and
     * rte_thread_register has no double-registration guard — it would silently
     * consume another RTE_MAX_LCORE slot — so gate on rte_lcore_id(). */
    if (LCORE_ID_ANY != rte_lcore_id())
    {
        return 0;
    }
    if (rte_thread_register() < 0)
    {
        return -1;
    }
    return 0;
}

int rusteron_dpdk_eal_thread_unregister(void)
{
    /* Never release an EAL lcore (e.g. the main lcore); only a NON_EAL lcore
     * acquired by rte_thread_register. Safe no-op on every other thread. */
    unsigned int lcore_id = rte_lcore_id();
    if (LCORE_ID_ANY != lcore_id && rte_lcore_has_role(lcore_id, ROLE_NON_EAL))
    {
        rte_thread_unregister();
    }
    return 0;
}

int rusteron_dpdk_eal_main_cpu(void)
{
    if (!rusteron_dpdk_eal_initialized)
    {
        return -1;
    }
    return (int)rte_lcore_to_cpu_id((int)rte_get_main_lcore());
}
