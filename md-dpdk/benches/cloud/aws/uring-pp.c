// SPDX-License-Identifier: Apache-2.0
//
// uring-pp: a kernel-UDP round trip between two hosts, to place io_uring's
// SQPOLL thread. The NRI lab (eks-nri.sh, phase X) runs one end in a latency
// pod on each node: the app thread on an exclusive CPU, and SQPOLL either on
// the pod's shared CPU, on a second exclusive CPU, or not at all (busy:
// non-blocking recv in a spin).
//
//   uring-pp ping <local_port> <peer_ip> <peer_port> <app_cpu> <busy|sqpoll> <sq_cpu|-> <seconds> <rate> <warmup> [label]
//   uring-pp pong <local_port> <peer_ip> <peer_port> <app_cpu> <busy|sqpoll> <sq_cpu|-> <seconds>
//
// ping sends 64-byte datagrams at <rate> a second (an absolute schedule; a
// late reply delays the next send, it is not dropped), waits for each echo
// spinning, and records the round trip (CLOCK_MONOTONIC). A reply not seen in
// 50 ms counts as lost. pong echoes every datagram until a 1-byte 'Q' or
// <seconds> pass. sqpoll mode keeps one RECV outstanding on an
// IORING_SETUP_SQPOLL|SQ_AFF ring (sq_thread_cpu = sq_cpu, sq_thread_idle
// 2 s) and reaps the CQ in a spin, so no syscall is made per message while
// the SQ thread is awake. No liburing: raw io_uring_setup/mmap.
//
// ping prints one CSV line, nanoseconds:
//   uringpp,<label>,<mode>,<samples>,<p50>,<p90>,<p99>,<p99.9>,<p99.99>,<max>,<mean>,<lost>,<nivcsw>,<app_cpu>,<sq_cpu>
// <nivcsw> is the app thread's involuntary context switches over the run.
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <errno.h>
#include <linux/io_uring.h>
#include <netinet/in.h>
#include <sched.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define MSG 64
#define SLOTS 16

static int64_t now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000000000LL + ts.tv_nsec;
}

static int cmp64(const void *a, const void *b) {
    int64_t x = *(const int64_t *)a, y = *(const int64_t *)b;
    return x < y ? -1 : x > y;
}

static void die(const char *what) {
    fprintf(stderr, "uring-pp: %s: %s\n", what, strerror(errno));
    exit(1);
}

// ---- io_uring, by hand -------------------------------------------------------

struct ring {
    int fd;
    unsigned *sq_head, *sq_tail, *sq_mask, *sq_array, *sq_flags, sq_entries;
    unsigned *cq_head, *cq_tail, *cq_mask;
    struct io_uring_sqe *sqes;
    struct io_uring_cqe *cqes;
};

static void ring_setup(struct ring *r, int sq_cpu) {
    struct io_uring_params p;
    memset(&p, 0, sizeof p);
    p.flags = IORING_SETUP_SQPOLL | IORING_SETUP_SQ_AFF;
    p.sq_thread_cpu = (unsigned)sq_cpu;
    p.sq_thread_idle = 2000;
    r->fd = (int)syscall(__NR_io_uring_setup, 16, &p);
    if (r->fd < 0) die("io_uring_setup (SQPOLL, SQ_AFF)");
    size_t sq_sz = p.sq_off.array + p.sq_entries * sizeof(unsigned);
    size_t cq_sz = p.cq_off.cqes + p.cq_entries * sizeof(struct io_uring_cqe);
    int single = (p.features & IORING_FEAT_SINGLE_MMAP) != 0;
    if (single && cq_sz > sq_sz) sq_sz = cq_sz;
    char *sq = mmap(NULL, sq_sz, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_POPULATE, r->fd, IORING_OFF_SQ_RING);
    if (sq == MAP_FAILED) die("mmap sq ring");
    char *cq = sq;
    if (!single) {
        cq = mmap(NULL, cq_sz, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_POPULATE, r->fd, IORING_OFF_CQ_RING);
        if (cq == MAP_FAILED) die("mmap cq ring");
    }
    r->sqes = mmap(NULL, p.sq_entries * sizeof(struct io_uring_sqe), PROT_READ | PROT_WRITE,
                   MAP_SHARED | MAP_POPULATE, r->fd, IORING_OFF_SQES);
    if (r->sqes == MAP_FAILED) die("mmap sqes");
    r->sq_head = (unsigned *)(sq + p.sq_off.head);
    r->sq_tail = (unsigned *)(sq + p.sq_off.tail);
    r->sq_mask = (unsigned *)(sq + p.sq_off.ring_mask);
    r->sq_array = (unsigned *)(sq + p.sq_off.array);
    r->sq_flags = (unsigned *)(sq + p.sq_off.flags);
    r->sq_entries = p.sq_entries;
    r->cq_head = (unsigned *)(cq + p.cq_off.head);
    r->cq_tail = (unsigned *)(cq + p.cq_off.tail);
    r->cq_mask = (unsigned *)(cq + p.cq_off.ring_mask);
    r->cqes = (struct io_uring_cqe *)(cq + p.cq_off.cqes);
}

static void ring_push(struct ring *r, int op, int fd, void *buf, unsigned len, uint64_t ud) {
    unsigned tail = *r->sq_tail;
    while (tail - __atomic_load_n(r->sq_head, __ATOMIC_ACQUIRE) >= r->sq_entries) {
    }
    unsigned idx = tail & *r->sq_mask;
    struct io_uring_sqe *s = &r->sqes[idx];
    memset(s, 0, sizeof *s);
    s->opcode = (unsigned char)op;
    s->fd = fd;
    s->addr = (uint64_t)(uintptr_t)buf;
    s->len = len;
    s->user_data = ud;
    r->sq_array[idx] = idx;
    __atomic_store_n(r->sq_tail, tail + 1, __ATOMIC_RELEASE);
    __atomic_thread_fence(__ATOMIC_SEQ_CST);
    if (__atomic_load_n(r->sq_flags, __ATOMIC_ACQUIRE) & IORING_SQ_NEED_WAKEUP)
        syscall(__NR_io_uring_enter, r->fd, 0, 0, IORING_ENTER_SQ_WAKEUP, NULL, 0);
}

static int ring_pop(struct ring *r, struct io_uring_cqe *out) {
    unsigned head = *r->cq_head;
    if (head == __atomic_load_n(r->cq_tail, __ATOMIC_ACQUIRE)) return 0;
    *out = r->cqes[head & *r->cq_mask];
    __atomic_store_n(r->cq_head, head + 1, __ATOMIC_RELEASE);
    return 1;
}

// ---- transport: busy recv or the ring ----------------------------------------

#define UD_RECV 1
#define UD_SEND 2

static int sock, uring_mode;
static struct ring R;
static char rbuf[2048];
static char sbuf[SLOTS][MSG];
static unsigned slot;
static long send_errors;

static void tx(const void *p, size_t len) {
    if (!uring_mode) {
        if (send(sock, p, len, 0) < 0) send_errors++;
        return;
    }
    char *b = sbuf[slot++ % SLOTS];
    memcpy(b, p, len);
    ring_push(&R, IORING_OP_SEND, sock, b, (unsigned)len, UD_SEND);
}

// one datagram into rbuf, or -1 if none is there yet
static int rx(void) {
    if (!uring_mode) {
        ssize_t n = recv(sock, rbuf, sizeof rbuf, MSG_DONTWAIT);
        return n >= 0 ? (int)n : -1;
    }
    struct io_uring_cqe c;
    while (ring_pop(&R, &c)) {
        if (c.user_data == UD_SEND) {
            if (c.res < 0) send_errors++;
            continue;
        }
        int n = c.res;
        if (n < 0) {
            errno = -n;
            die("io_uring recv");
        }
        // copy out before the next RECV can land in rbuf
        static char hold[2048];
        memcpy(hold, rbuf, (size_t)n);
        ring_push(&R, IORING_OP_RECV, sock, rbuf, sizeof rbuf, UD_RECV);
        memcpy(rbuf, hold, (size_t)n);
        return n;
    }
    return -1;
}

int main(int argc, char **argv) {
    if (argc < 9) {
        fprintf(stderr, "usage: uring-pp ping|pong <local_port> <peer_ip> <peer_port> <app_cpu> <busy|sqpoll> <sq_cpu|-> <seconds> [rate warmup label]\n");
        return 2;
    }
    int ping = strcmp(argv[1], "ping") == 0;
    int lport = atoi(argv[2]), pport = atoi(argv[4]), app_cpu = atoi(argv[5]);
    uring_mode = strcmp(argv[6], "sqpoll") == 0;
    int sq_cpu = strcmp(argv[7], "-") == 0 ? -1 : atoi(argv[7]);
    double seconds = atof(argv[8]);
    long rate = argc > 9 ? atol(argv[9]) : 10000;
    long warmup = argc > 10 ? atol(argv[10]) : 10000;
    const char *label = argc > 11 ? argv[11] : "-";
    if (uring_mode && sq_cpu < 0) {
        fprintf(stderr, "uring-pp: sqpoll needs an sq_cpu\n");
        return 2;
    }

    cpu_set_t set;
    CPU_ZERO(&set);
    CPU_SET(app_cpu, &set);
    if (sched_setaffinity(0, sizeof set, &set) != 0) die("pin the app thread");

    sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) die("socket");
    int one = 1;
    setsockopt(sock, SOL_SOCKET, SO_REUSEADDR, &one, sizeof one);
    struct sockaddr_in me = {.sin_family = AF_INET, .sin_port = htons((uint16_t)lport), .sin_addr.s_addr = INADDR_ANY};
    if (bind(sock, (struct sockaddr *)&me, sizeof me) != 0) die("bind");
    struct sockaddr_in peer = {.sin_family = AF_INET, .sin_port = htons((uint16_t)pport)};
    if (inet_pton(AF_INET, argv[3], &peer.sin_addr) != 1) {
        fprintf(stderr, "uring-pp: peer ip %s\n", argv[3]);
        return 2;
    }
    if (connect(sock, (struct sockaddr *)&peer, sizeof peer) != 0) die("connect");
    if (uring_mode) {
        ring_setup(&R, sq_cpu);
        ring_push(&R, IORING_OP_RECV, sock, rbuf, sizeof rbuf, UD_RECV);
    }

    struct rusage ru0, ru1;
    getrusage(RUSAGE_THREAD, &ru0);
    int64_t t_end = now_ns() + (int64_t)(seconds * 1e9);

    if (!ping) {
        long echoed = 0;
        while (now_ns() < t_end) {
            int n = rx();
            if (n < 0) continue;
            if (n == 1 && rbuf[0] == 'Q') break;
            tx(rbuf, (size_t)n);
            echoed++;
        }
        getrusage(RUSAGE_THREAD, &ru1);
        fprintf(stderr, "uring-pp pong: %ld echoed, %ld send errors, nivcsw %ld, cpu %d\n", echoed, send_errors,
                ru1.ru_nivcsw - ru0.ru_nivcsw, sched_getcpu());
        return 0;
    }

    size_t cap = (size_t)(seconds * (double)rate) + 16;
    int64_t *rtt = malloc(cap * sizeof *rtt);
    if (!rtt) return 1;
    size_t n = 0;
    long lost = 0, seq = 0, bad_cpu = 0;
    int64_t interval = 1000000000LL / (rate > 0 ? rate : 1);
    int64_t next = now_ns();
    char msg[MSG];
    memset(msg, 0, sizeof msg);
    while (n < cap) {
        int64_t t;
        while ((t = now_ns()) < next) {
        }
        if (t >= t_end) break;
        seq++;
        memcpy(msg, &seq, sizeof seq);
        int64_t t0 = now_ns();
        tx(msg, sizeof msg);
        for (;;) {
            int got = rx();
            int64_t t1 = now_ns();
            if (got >= (int)sizeof seq) {
                long rs;
                memcpy(&rs, rbuf, sizeof rs);
                if (rs != seq) continue; // a late echo of a lost one
                if (seq > warmup) rtt[n++] = t1 - t0;
                break;
            }
            if (t1 - t0 > 50000000LL) {
                lost++;
                break;
            }
        }
        if (sched_getcpu() != app_cpu) bad_cpu++;
        next += interval;
        if (next < now_ns() - 10 * interval) next = now_ns(); // fell far behind: restart the schedule
    }
    getrusage(RUSAGE_THREAD, &ru1);
    for (int i = 0; i < 3; i++) {
        tx("Q", 1);
        usleep(1000);
    }
    if (n == 0) {
        fprintf(stderr, "uring-pp: no samples (lost %ld)\n", lost);
        return 1;
    }
    double sum = 0;
    for (size_t i = 0; i < n; i++) sum += (double)rtt[i];
    qsort(rtt, n, sizeof *rtt, cmp64);
#define P(p) (long long)rtt[(size_t)((p) * (double)(n - 1) + 0.5)]
    printf("uringpp,%s,%s,%zu,%lld,%lld,%lld,%lld,%lld,%lld,%.0f,%ld,%ld,%d,%d\n", label, argv[6], n, P(0.50), P(0.90),
           P(0.99), P(0.999), P(0.9999), (long long)rtt[n - 1], sum / (double)n, lost, ru1.ru_nivcsw - ru0.ru_nivcsw,
           app_cpu, sq_cpu);
#undef P
    if (bad_cpu) fprintf(stderr, "uring-pp: %ld samples ran off cpu %d\n", bad_cpu, app_cpu);
    if (send_errors) fprintf(stderr, "uring-pp: %ld send errors\n", send_errors);
    return 0;
}
