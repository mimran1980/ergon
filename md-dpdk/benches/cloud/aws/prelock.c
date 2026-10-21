/* prelock [--mlock] -- <cmd> [args...]: prepare the command's memory lock, then
 * exec it. Real-time scheduling is deliberately absent (no SCHED_FIFO, no
 * --fifo, no sched_setscheduler): the one AWS A/B was inconclusive and kept
 * the CPU's tick on (io/benches/BENCHMARK.md, Host and NIC settings; RT
 * throttle and RT-group-sched bite it).
 *
 * mlockall(2) does NOT survive execve(2): the address space it locked is the
 * one exec replaces, so a wrapper that locks and then execs leaves the target
 * with VmLck: 0 kB. The lock has to be taken by the process that keeps it.
 * --mlock therefore asks the target to take it, by putting
 * RUSTERON_MEDIA_DRIVER_MLOCKALL=1 in its environment (the media driver and
 * the lab clients call lock_memory_from_env() and then verify their own
 * VmLck); this wrapper never claims a lock it cannot pass on.
 *
 * The latency pod stays unprivileged: the NRI plugin's RLIMIT_MEMLOCK grant
 * (rlimits.ergon.io/app: "memlock") is what makes mlockall work without
 * IPC_LOCK or CAP_SYS_NICE. Rlimits do survive exec, so this wrapper checks
 * the grant is in place before handing over.
 *
 *   --mlock          ask the target to mlockall; also from
 *                    ERGON_PRELOCK_MLOCK=1
 *
 * What it did is one line on stdout. A requested operation that fails is a
 * typed error on stderr and exit 1 (fail closed): the benchmark records the
 * missing grant, it never runs as if the grant were there. Nothing here is a
 * capability: with the plugin's RLIMIT_MEMLOCK=inf an ordinary user may
 * mlockall.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <unistd.h>

static void die(const char *what)
{
    fprintf(stderr, "prelock: %s: %s\n", what, strerror(errno));
    exit(1);
}

int main(int argc, char **argv)
{
    int mlock = 0, i = 1;
    const char *env;
    struct rlimit lim;

    env = getenv("ERGON_PRELOCK_MLOCK");
    if (env && *env && strcmp(env, "0") != 0)
        mlock = 1;

    for (; i < argc; i++) {
        if (strcmp(argv[i], "--") == 0) {
            i++;
            break;
        }
        if (strcmp(argv[i], "--mlock") == 0) {
            mlock = 1;
        } else {
            fprintf(stderr, "prelock: %s: want --mlock or -- <cmd>\n", argv[i]);
            return 2;
        }
    }
    if (i >= argc) {
        fprintf(stderr, "prelock: no command after --\n");
        return 2;
    }

    if (mlock) {
        if (getrlimit(RLIMIT_MEMLOCK, &lim) != 0)
            die("getrlimit(RLIMIT_MEMLOCK)");
        if (lim.rlim_cur == 0) {
            /* Fail closed: the target's mlockall would fail on every page and
             * the run would look like a measured one. */
            fprintf(stderr,
                    "prelock: RLIMIT_MEMLOCK is 0, so the target cannot mlockall; "
                    "need the NRI plugin's rlimits.ergon.io/app: \"memlock\"\n");
            return 1;
        }
        if (lim.rlim_cur == RLIM_INFINITY) {
            printf("prelock: RLIMIT_MEMLOCK unlimited; asking the target to lock memory\n");
        } else {
            printf("prelock: RLIMIT_MEMLOCK %llu bytes; asking the target to lock memory\n",
                   (unsigned long long)lim.rlim_cur);
        }
        /* The lock itself: by the target, which verifies its own VmLck. */
        if (setenv("RUSTERON_MEDIA_DRIVER_MLOCKALL", "1", 1) != 0)
            die("setenv(RUSTERON_MEDIA_DRIVER_MLOCKALL=1)");
    }
    fflush(stdout);
    execvp(argv[i], &argv[i]);
    fprintf(stderr, "prelock: exec %s: %s\n", argv[i], strerror(errno));
    return 1;
}
