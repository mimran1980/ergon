// SPDX-License-Identifier: Apache-2.0
//
// wakelat: a sleepy helper's wake-up wait. The NRI lab (eks-nri.sh, phase X)
// runs it inside a latency pod, where the ergon-cpus hook starts every thread
// on the pod's shared CPUs (ERGON_SHARED_CPUS) with a 1 ns timer slack.
//
//   wakelat <seconds> <period_us> <slack_ns> [label]
//
// Each wake-up sleeps to an absolute CLOCK_MONOTONIC deadline
// (clock_nanosleep TIMER_ABSTIME) and records how late it woke: the timer
// slack plus the time the woken thread waited for its CPU. slack_ns -1
// leaves the thread's slack as it found it (the hook's 1 ns in a plugin pod,
// the kernel's 50 us elsewhere); any other value is set with
// PR_SET_TIMERSLACK first (0 means "the default" to the kernel, so pass 1
// for 1 ns). Every wake-up also checks sched_getcpu() against
// ERGON_SHARED_CPUS when that is set: a helper that woke on a CPU outside it
// is counted (offshared).
//
// One CSV line on stdout, nanoseconds:
//   wakelat,<label>,<samples>,<p50>,<p90>,<p99>,<p99.9>,<p99.99>,<max>,<mean>,<offshared>,<slack_ns>,<cpus_seen>
#define _GNU_SOURCE
#include <errno.h>
#include <sched.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <time.h>

static int64_t now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000000000LL + ts.tv_nsec;
}

static int cmp64(const void *a, const void *b) {
    int64_t x = *(const int64_t *)a, y = *(const int64_t *)b;
    return x < y ? -1 : x > y;
}

// "0,2-3" -> bitmask of up to 1024 CPUs
static int parse_cpus(const char *s, unsigned char *mask, int max) {
    int n = 0;
    while (s && *s) {
        char *end;
        long lo = strtol(s, &end, 10), hi = lo;
        if (end == s) return -1;
        if (*end == '-') {
            s = end + 1;
            hi = strtol(s, &end, 10);
            if (end == s) return -1;
        }
        for (long c = lo; c <= hi && c < max; c++) {
            mask[c] = 1;
            n++;
        }
        s = *end == ',' ? end + 1 : end;
        if (*end && *end != ',') return -1;
    }
    return n;
}

static int64_t pct(const int64_t *v, size_t n, double p) {
    size_t i = (size_t)(p * (double)(n - 1) + 0.5);
    return v[i < n ? i : n - 1];
}

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: wakelat <seconds> <period_us> <slack_ns|-1> [label]\n");
        return 2;
    }
    double seconds = atof(argv[1]);
    long period_us = atol(argv[2]);
    long slack = atol(argv[3]);
    const char *label = argc > 4 ? argv[4] : "-";
    if (seconds <= 0 || period_us <= 0) {
        fprintf(stderr, "wakelat: seconds and period_us must be positive\n");
        return 2;
    }
    if (slack >= 0 && prctl(PR_SET_TIMERSLACK, (unsigned long)slack, 0, 0, 0) != 0) {
        fprintf(stderr, "wakelat: PR_SET_TIMERSLACK %ld: %s\n", slack, strerror(errno));
        return 1;
    }
    long got_slack = prctl(PR_GET_TIMERSLACK, 0, 0, 0, 0);
    unsigned char shared[1024] = {0};
    const char *env = getenv("ERGON_SHARED_CPUS");
    int have_shared = env && *env && parse_cpus(env, shared, 1024) > 0;
    unsigned char seen[1024] = {0};

    size_t cap = (size_t)(seconds * 1e6 / (double)period_us) + 16;
    int64_t *late = malloc(cap * sizeof *late);
    if (!late) return 1;
    size_t n = 0;
    long offshared = 0;
    int64_t period = period_us * 1000LL;
    int64_t start = now_ns(), end = start + (int64_t)(seconds * 1e9);
    int64_t deadline = start + period;
    while (n < cap) {
        struct timespec ts = {.tv_sec = deadline / 1000000000LL, .tv_nsec = deadline % 1000000000LL};
        int rc;
        while ((rc = clock_nanosleep(CLOCK_MONOTONIC, TIMER_ABSTIME, &ts, NULL)) == EINTR) {
        }
        if (rc != 0) {
            fprintf(stderr, "wakelat: clock_nanosleep: %s\n", strerror(rc));
            return 1;
        }
        int64_t t = now_ns();
        late[n++] = t - deadline;
        int cpu = sched_getcpu();
        if (cpu >= 0 && cpu < 1024) {
            seen[cpu] = 1;
            if (have_shared && !shared[cpu]) offshared++;
        }
        if (t >= end) break;
        deadline += period;
        if (deadline < t) deadline = t + period; // overran a whole period: restart the grid
    }
    if (n == 0) return 1;
    double sum = 0;
    for (size_t i = 0; i < n; i++) sum += (double)late[i];
    qsort(late, n, sizeof *late, cmp64);
    char cpus[4096] = "";
    size_t off = 0;
    for (int c = 0; c < 1024; c++)
        if (seen[c]) off += (size_t)snprintf(cpus + off, sizeof cpus - off, "%s%d", off ? " " : "", c);
    printf("wakelat,%s,%zu,%lld,%lld,%lld,%lld,%lld,%lld,%.0f,%ld,%ld,%s\n", label, n,
           (long long)pct(late, n, 0.50), (long long)pct(late, n, 0.90), (long long)pct(late, n, 0.99),
           (long long)pct(late, n, 0.999), (long long)pct(late, n, 0.9999), (long long)late[n - 1],
           sum / (double)n, have_shared ? offshared : -1L, got_slack, cpus);
    return 0;
}
