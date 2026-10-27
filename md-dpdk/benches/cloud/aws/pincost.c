// SPDX-License-Identifier: Apache-2.0
//
// pincost: what reading the plugin's CPU env and pinning a thread cost. The
// NRI lab (eks-nri.sh, phase X) runs it in a latency pod. It does what
// k8s/nri-shared-cpus/examples/pin.rs's pin_isolated(i) does: read
// ERGON_ISOLATED_CPUS, take its i-th CPU, sched_setaffinity to it.
//
//   pincost <iterations>
//
// Per iteration, three timings (CLOCK_MONOTONIC, ns):
//   env    getenv("ERGON_ISOLATED_CPUS") and parse it into a CPU list
//   pin    the sched_setaffinity call that moves the thread from its shared
//          CPUs to the first isolated CPU
//   moved  from before that call until sched_getcpu() first reports the
//          isolated CPU (the call plus the migration)
// then the thread goes back to the shared CPUs (untimed), so each iteration
// pays a real migration. Pinning is a start-up step, not a hot-path one: this
// says what the step costs, once.
//
// One CSV line per timing on stdout:
//   pincost,<what>,<samples>,<p50>,<p90>,<p99>,<p99.9>,<max>,<mean>
#define _GNU_SOURCE
#include <errno.h>
#include <sched.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
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

static int parse_list(const char *s, int *out, int max) {
    int n = 0;
    while (s && *s) {
        char *end;
        long lo = strtol(s, &end, 10), hi = lo;
        if (end == s) return -1;
        if (*end == '-') {
            const char *h = end + 1;
            hi = strtol(h, &end, 10);
            if (end == h) return -1;
        }
        for (long c = lo; c <= hi && n < max; c++) out[n++] = (int)c;
        if (*end && *end != ',') return -1;
        s = *end ? end + 1 : end;
    }
    return n;
}

static void report(const char *what, int64_t *v, size_t n) {
    double sum = 0;
    for (size_t i = 0; i < n; i++) sum += (double)v[i];
    qsort(v, n, sizeof *v, cmp64);
#define P(p) (long long)v[(size_t)((p) * (double)(n - 1) + 0.5)]
    printf("pincost,%s,%zu,%lld,%lld,%lld,%lld,%lld,%.0f\n", what, n, P(0.50), P(0.90), P(0.99), P(0.999),
           (long long)v[n - 1], sum / (double)n);
#undef P
}

int main(int argc, char **argv) {
    long iters = argc > 1 ? atol(argv[1]) : 10000;
    if (iters <= 0) return 2;
    const char *iso_env = getenv("ERGON_ISOLATED_CPUS");
    const char *sh_env = getenv("ERGON_SHARED_CPUS");
    int iso[1024], sh[1024];
    int niso = parse_list(iso_env, iso, 1024), nsh = parse_list(sh_env, sh, 1024);
    if (niso <= 0 || nsh <= 0) {
        fprintf(stderr, "pincost: needs ERGON_ISOLATED_CPUS and ERGON_SHARED_CPUS (got '%s' and '%s')\n",
                iso_env ? iso_env : "", sh_env ? sh_env : "");
        return 1;
    }
    cpu_set_t shared, target;
    CPU_ZERO(&shared);
    for (int i = 0; i < nsh; i++) CPU_SET(sh[i], &shared);
    int64_t *env = malloc((size_t)iters * sizeof *env), *pin = malloc((size_t)iters * sizeof *pin),
            *moved = malloc((size_t)iters * sizeof *moved);
    if (!env || !pin || !moved) return 1;
    volatile int sink = 0;
    for (long i = 0; i < iters; i++) {
        if (sched_setaffinity(0, sizeof shared, &shared) != 0) {
            fprintf(stderr, "pincost: back to shared: %s\n", strerror(errno));
            return 1;
        }
        while (!CPU_ISSET(sched_getcpu(), &shared)) {
        }
        int64_t t0 = now_ns();
        int list[1024];
        int n = parse_list(getenv("ERGON_ISOLATED_CPUS"), list, 1024);
        int64_t t1 = now_ns();
        sink += n > 0 ? list[0] : 0;
        CPU_ZERO(&target);
        CPU_SET(iso[0], &target);
        int64_t t2 = now_ns();
        if (sched_setaffinity(0, sizeof target, &target) != 0) {
            fprintf(stderr, "pincost: pin to %d: %s\n", iso[0], strerror(errno));
            return 1;
        }
        int64_t t3 = now_ns();
        while (sched_getcpu() != iso[0]) {
        }
        int64_t t4 = now_ns();
        env[i] = t1 - t0;
        pin[i] = t3 - t2;
        moved[i] = t4 - t2;
    }
    (void)sink;
    report("env", env, (size_t)iters);
    report("pin", pin, (size_t)iters);
    report("moved", moved, (size_t)iters);
    return 0;
}
