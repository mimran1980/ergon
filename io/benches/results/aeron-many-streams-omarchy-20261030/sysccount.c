// LD_PRELOAD shim: counts socket receive/send and epoll calls per thread, printed at exit.
// For counting what Aeron's driver agents do per duty cycle; adds a few ns per call.
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

#define SLOTS 256
enum { RECVMMSG, RECVMSG, RECVFROM, EPOLL_WAIT, SENDMMSG, SENDMSG, SENDTO, KINDS };
static const char *names[KINDS] = {"recvmmsg", "recvmsg", "recvfrom", "epoll_wait", "sendmmsg", "sendmsg", "sendto"};
static _Atomic int tids[SLOTS];
static _Atomic long counts[SLOTS][KINDS];
static __thread int slot = -1;

static void count(int kind)
{
    if (slot < 0)
    {
        int tid = (int)syscall(SYS_gettid);
        for (int i = 0; i < SLOTS; i++)
        {
            int expected = 0;
            if (atomic_compare_exchange_strong(&tids[i], &expected, tid)) { slot = i; break; }
        }
        if (slot < 0) return;
    }
    atomic_fetch_add_explicit(&counts[slot][kind], 1, memory_order_relaxed);
}

#define REAL(name) static __typeof__(name) *real_##name; if (!real_##name) real_##name = dlsym(RTLD_NEXT, #name)

int recvmmsg(int fd, struct mmsghdr *v, unsigned int n, int flags, struct timespec *t)
{ REAL(recvmmsg); count(RECVMMSG); return real_recvmmsg(fd, v, n, flags, t); }
ssize_t recvmsg(int fd, struct msghdr *m, int flags)
{ REAL(recvmsg); count(RECVMSG); return real_recvmsg(fd, m, flags); }
ssize_t recvfrom(int fd, void *b, size_t l, int flags, struct sockaddr *a, socklen_t *al)
{ REAL(recvfrom); count(RECVFROM); return real_recvfrom(fd, b, l, flags, a, al); }
int epoll_wait(int e, struct epoll_event *ev, int max, int timeout)
{ REAL(epoll_wait); count(EPOLL_WAIT); return real_epoll_wait(e, ev, max, timeout); }
int sendmmsg(int fd, struct mmsghdr *v, unsigned int n, int flags)
{ REAL(sendmmsg); count(SENDMMSG); return real_sendmmsg(fd, v, n, flags); }
ssize_t sendmsg(int fd, const struct msghdr *m, int flags)
{ REAL(sendmsg); count(SENDMSG); return real_sendmsg(fd, m, flags); }
ssize_t sendto(int fd, const void *b, size_t l, int flags, const struct sockaddr *a, socklen_t al)
{ REAL(sendto); count(SENDTO); return real_sendto(fd, b, l, flags, a, al); }

__attribute__((destructor)) static void dump(void)
{
    const char *dir = getenv("SYSCOUNT_DIR");
    if (!dir) return;
    char path[4096];
    snprintf(path, sizeof(path), "%s/syscalls.%d", dir, (int)getpid());
    FILE *f = fopen(path, "w");
    if (!f) return;
    for (int i = 0; i < SLOTS; i++)
    {
        int tid = atomic_load(&tids[i]);
        if (!tid) continue;
        fprintf(f, "tid %d", tid);
        for (int k = 0; k < KINDS; k++) fprintf(f, " %s=%ld", names[k], atomic_load(&counts[i][k]));
        fprintf(f, "\n");
    }
    fclose(f);
}
