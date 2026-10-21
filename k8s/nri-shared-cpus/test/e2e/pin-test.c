/* SPDX-License-Identifier: Apache-2.0 */
/*
 * pin-test: the e2e's in-pod test program.
 *
 *   pin-test hold            print what the OCI hook gave us (own affinity, a
 *                            new pthread's affinity, 1000 sched_getcpu()
 *                            samples), then pin a thread to isolated:0 and run
 *                            a helper that wakes every 10 ms and checks
 *                            sched_getcpu() is in the shared list. Prints
 *                            "VIOLATION" on every check that fails and a
 *                            "SUMMARY" line on SIGTERM.
 *   pin-test sample I SEC    pin to isolated index I and sample sched_getcpu()
 *                            for SEC seconds. Prints one SAMPLE line.
 *   pin-test probe           print the container's CPU view as key=value lines.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

static volatile sig_atomic_t running = 1;
static long checks = 0, violations = 0;

static void on_term(int sig) { (void)sig; running = 0; }

static int nth_cpu(const char *list, int i)
{
    if (!list) return -1;
    for (const char *p = list; *p != '\0'; i--) {
        char *end;
        long cpu = strtol(p, &end, 10);
        if (end == p) return -1;
        if (i == 0) return (int)cpu;
        p = (*end == ',') ? end + 1 : "";
    }
    return -1;
}

static int in_list(const char *list, int cpu)
{
    if (!list) return 0;
    for (const char *p = list; *p != '\0';) {
        char *end;
        long c = strtol(p, &end, 10);
        if (end == p) return 0;
        if ((int)c == cpu) return 1;
        p = (*end == ',') ? end + 1 : "";
    }
    return 0;
}

static int pin_cpu(int cpu)
{
    cpu_set_t set;
    CPU_ZERO(&set);
    CPU_SET(cpu, &set);
    return sched_setaffinity(0, sizeof set, &set);
}

static void format_affinity(char *out, size_t n)
{
    cpu_set_t set;
    CPU_ZERO(&set);
    if (sched_getaffinity(0, sizeof set, &set) != 0) {
        snprintf(out, n, "ERR%d", errno);
        return;
    }
    size_t used = 0;
    out[0] = '\0';
    int first = 1;
    for (int c = 0; c < CPU_SETSIZE && used + 8 < n; c++) {
        if (CPU_ISSET(c, &set)) {
            int w = snprintf(out + used, n - used, "%s%d", first ? "" : ",", c);
            if (w < 0) break;
            used += (size_t)w;
            first = 0;
        }
    }
    if (first) snprintf(out, n, "(empty)");
}

static void report_affinity(const char *tag)
{
    char buf[512];
    format_affinity(buf, sizeof buf);
    printf("%s.affinity=%s\n", tag, buf);
}

/* Cpus_allowed_list of pid 1 (the container's init), not of this process. */
static void report_pid1(void)
{
    FILE *f = fopen("/proc/1/status", "r");
    if (!f) {
        printf("pid1.allowed=ERR%d\n", errno);
        return;
    }
    char line[256];
    while (fgets(line, sizeof line, f)) {
        if (strncmp(line, "Cpus_allowed_list:", 18) == 0) {
            char *v = line + 18;
            while (*v == '\t' || *v == ' ') v++;
            v[strcspn(v, "\n")] = '\0';
            printf("pid1.allowed=%s\n", v);
            fclose(f);
            return;
        }
    }
    fclose(f);
    printf("pid1.allowed=MISSING\n");
}

static int do_probe(void)
{
    const char *iso = getenv("ERGON_ISOLATED_CPUS");
    const char *sh = getenv("ERGON_SHARED_CPUS");
    const char *cores = getenv("ERGON_ISOLATED_CORES");
    const char *milli = getenv("ERGON_SHARED_MILLICPUS");
    const char *helper = getenv("ERGON_HELPER_CPUS");
    const char *oi = getenv("OPENSHIFT_ISOLATED_CPUS");
    const char *os_ = getenv("OPENSHIFT_SHARED_CPUS");
    printf("ERGON_ISOLATED_CPUS=%s\n", iso ? iso : "");
    printf("ERGON_SHARED_CPUS=%s\n", sh ? sh : "");
    printf("ERGON_ISOLATED_CORES=%s\n", cores ? cores : "");
    printf("ERGON_SHARED_MILLICPUS=%s\n", milli ? milli : "");
    printf("ERGON_HELPER_CPUS=%s\n", helper ? helper : "");
    printf("OPENSHIFT_ISOLATED_CPUS=%s\n", oi ? oi : "");
    printf("OPENSHIFT_SHARED_CPUS=%s\n", os_ ? os_ : "");

    char buf[4096];
    FILE *f = fopen("/sys/fs/cgroup/cpuset.cpus.effective", "r");
    if (f && fgets(buf, sizeof buf, f)) {
        buf[strcspn(buf, "\n")] = '\0';
        printf("cpuset.cpus.effective=%s\n", buf);
    } else {
        printf("cpuset.cpus.effective=ERR%d\n", errno);
    }
    if (f) fclose(f);

    f = fopen("/sys/fs/cgroup/cpu.max", "r");
    if (f && fgets(buf, sizeof buf, f)) {
        buf[strcspn(buf, "\n")] = '\0';
        printf("cpu.max=%s\n", buf);
    } else {
        printf("cpu.max=ERR%d\n", errno);
    }
    if (f) fclose(f);

    report_affinity("self");
    report_pid1();
    printf("sched_getcpu=%d\n", sched_getcpu());
    return 0;
}

/* Prints the affinity a brand-new thread inherits, then exits. */
static void *affinity_probe(void *arg)
{
    (void)arg;
    report_affinity("pthread");
    return NULL;
}

static void *helper_main(void *arg)
{
    const char *sh = (const char *)arg;
    while (running) {
        int now = sched_getcpu();
        checks++;
        if (!in_list(sh, now)) {
            violations++;
            printf("VIOLATION helper on cpu %d, not in %s\n", now, sh);
            fflush(stdout);
        }
        usleep(10000);
    }
    return NULL;
}

/* What the OCI hook gave PID 1, before we pin anything. */
/* Timer slack of this process and of a brand-new thread. The OCI hook writes
 * /proc/<pid>/timerslack_ns; the kernel copies the value to every thread and
 * child made afterwards (copy_process), so the new thread must show it too.
 * 1 ns is the hook's value; the kernel's default is 50000.
 *
 * The thread's value is read from its own task directory, not from
 * /proc/self. /proc/self resolves to the thread-group leader (fs/proc/self.c,
 * task_tgid_nr_ns), and reading another task's timerslack_ns needs
 * CAP_SYS_NICE over that task (timerslack_ns_show, fs/proc/base.c: the only
 * exception is `p == current`, and a sibling thread is not `current`). So a
 * non-leader thread reading /proc/self/timerslack_ns gets the leader's file
 * and is refused with EPERM. /proc/thread-self or /proc/<own tid> is `p ==
 * current` and is free. This probe reads both and reports each path's errno,
 * so a failure is a diagnosis and not a mystery. */
static long read_slack(const char *path, char *out, size_t n, int *err)
{
    *err = 0;
    int fd = open(path, O_RDONLY);
    if (fd < 0) {
        *err = errno;
        return -1;
    }
    char buf[64];
    ssize_t got = read(fd, buf, sizeof buf - 1);
    if (got < 0) {
        *err = errno;
        close(fd);
        return -1;
    }
    close(fd);
    buf[got] = '\0';
    buf[strcspn(buf, "\n")] = '\0';
    snprintf(out, n, "%s", buf);
    return got;
}

static void *slack_probe(void *arg)
{
    (void)arg;
    char tidpath[64], selfval[64] = "?", tidval[64] = "?";
    int selferr = 0, tiderr = 0;
    long tid = syscall(SYS_gettid);
    snprintf(tidpath, sizeof tidpath, "/proc/%ld/timerslack_ns", tid);
    long selfn = read_slack("/proc/self/timerslack_ns", selfval, sizeof selfval, &selferr);
    long tidn = read_slack(tidpath, tidval, sizeof tidval, &tiderr);

    /* One line for the e2e's check: the value on this brand-new thread. */
    if (tiderr == 0 && tidn > 0) {
        printf("timerslack.pthread=%s\n", tidval);
    } else if (selferr == 0 && selfn > 0) {
        printf("timerslack.pthread=%s\n", selfval);
    } else {
        printf("timerslack.pthread=EMPTY\n");
    }
    /* How each read went, kept off the checked line's prefix. `self` is the
     * leader's file from here and is refused without CAP_SYS_NICE; `via_tid`
     * is this thread's own and is free. */
    printf("slack_probe: tid=%ld self=%s n=%ld err=%d via_tid=%s n=%ld err=%d\n",
           tid, selfval, selfn, selferr, tidval, tidn, tiderr);
    return NULL;
}

static void report_timer_slack(void)
{
    FILE *f = fopen("/proc/self/timerslack_ns", "r");
    if (!f) {
        printf("timerslack.self=ERR%d\n", errno);
    } else {
        char buf[64];
        if (fgets(buf, sizeof buf, f)) {
            buf[strcspn(buf, "\n")] = '\0';
            printf("timerslack.self=%s\n", buf);
        } else {
            printf("timerslack.self=EMPTY\n");
        }
        fclose(f);
    }
    pthread_t probe;
    if (pthread_create(&probe, NULL, slack_probe, NULL) != 0) {
        perror("pin-test: pthread_create(slack_probe)");
        return;
    }
    pthread_join(probe, NULL);
}

static void report_hook_effect(const char *sh)
{
    report_affinity("init");
    report_timer_slack();
    pthread_t probe;
    if (pthread_create(&probe, NULL, affinity_probe, NULL) != 0) {
        perror("pin-test: pthread_create(probe)");
        return;
    }
    pthread_join(probe, NULL);

    int off = 0;
    char seen[64];
    size_t used = 0;
    seen[0] = '\0';
    for (int i = 0; i < 1000; i++) {
        int c = sched_getcpu();
        if (!in_list(sh, c)) off++;
        if (!in_list(seen, c) && used + 8 < sizeof seen) {
            int w = snprintf(seen + used, sizeof seen - used, "%s%d",
                             used ? "," : "", c);
            if (w > 0) used += (size_t)w;
        }
        /* Busy enough to move if the mask were wider. */
        for (volatile int s = 0; s < 20000; s++) {}
    }
    printf("samples=1000 off_shared=%d cpus=%s\n", off, used ? seen : "(none)");
}

static int do_sample(const char *idx_text, const char *sec_text)
{
    const char *iso = getenv("ERGON_ISOLATED_CPUS");
    if (!iso) {
        fprintf(stderr, "pin-test sample: ERGON_ISOLATED_CPUS must be set\n");
        return 1;
    }
    int idx = atoi(idx_text);
    int sec = atoi(sec_text);
    if (sec < 1) {
        fprintf(stderr, "pin-test sample: seconds must be >= 1\n");
        return 1;
    }
    int want = nth_cpu(iso, idx);
    if (want < 0) {
        fprintf(stderr, "pin-test sample: no isolated CPU at index %d in %s\n", idx, iso);
        return 1;
    }
    if (pin_cpu(want) != 0) {
        perror("pin-test: sched_setaffinity");
        return 1;
    }
    struct timespec end, now;
    clock_gettime(CLOCK_MONOTONIC, &end);
    end.tv_sec += sec;
    long n = 0, bad = 0;
    for (;;) {
        int c = sched_getcpu();
        n++;
        if (c != want) bad++;
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (now.tv_sec > end.tv_sec ||
            (now.tv_sec == end.tv_sec && now.tv_nsec >= end.tv_nsec))
            break;
    }
    printf("SAMPLE idx=%d cpu=%d n=%ld bad=%ld\n", idx, want, n, bad);
    return bad == 0 ? 0 : 1;
}

static int do_hold(void)
{
    const char *iso = getenv("ERGON_ISOLATED_CPUS");
    const char *sh = getenv("ERGON_SHARED_CPUS");
    if (!iso || !sh) {
        fprintf(stderr, "pin-test hold: ERGON_ISOLATED_CPUS and ERGON_SHARED_CPUS must be set\n");
        return 1;
    }
    int hot = nth_cpu(iso, 0);
    if (hot < 0) {
        fprintf(stderr, "pin-test hold: no isolated CPU in %s\n", iso);
        return 1;
    }

    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = on_term;
    sigaction(SIGTERM, &sa, NULL);
    sigaction(SIGINT, &sa, NULL);

    /* Step 5 evidence, before any pin: what the hook left us with. */
    report_hook_effect(sh);
    fflush(stdout);

    /* Start the helper first: a new thread copies its creator's mask, so it
     * must be created while we are still on the shared CPUs. */
    pthread_t helper;
    if (pthread_create(&helper, NULL, helper_main, (void *)sh) != 0) {
        perror("pin-test: pthread_create");
        return 1;
    }

    if (pin_cpu(hot) != 0) {
        perror("pin-test: sched_setaffinity");
        return 1;
    }
    printf("pin-test: hot thread pinned to isolated index 0 = cpu %d\n", hot);
    printf("pin-test: shared list is %s\n", sh);
    fflush(stdout);

    int hot_violations = 0;
    while (running) {
        int hot_now = sched_getcpu();
        if (hot_now != hot) {
            hot_violations++;
            printf("VIOLATION hot thread moved to cpu %d, expected %d\n", hot_now, hot);
            fflush(stdout);
        }
        usleep(10000);
    }
    pthread_join(helper, NULL);
    printf("SUMMARY checks=%ld violations=%ld hot_violations=%d\n", checks, violations, hot_violations);
    fflush(stdout);
    return (violations == 0 && hot_violations == 0) ? 0 : 1;
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr, "usage: pin-test probe|hold|sample <index> <seconds>\n");
        return 2;
    }
    if (strcmp(argv[1], "probe") == 0) return do_probe();
    if (strcmp(argv[1], "hold") == 0) return do_hold();
    if (strcmp(argv[1], "sample") == 0) {
        if (argc < 4) {
            fprintf(stderr, "usage: pin-test sample <index> <seconds>\n");
            return 2;
        }
        return do_sample(argv[2], argv[3]);
    }
    fprintf(stderr, "pin-test: unknown mode %s\n", argv[1]);
    return 2;
}
