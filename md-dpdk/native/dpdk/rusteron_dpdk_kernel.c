/*
 * ergon-md-dpdk DPDK transport — the kernel path for host-local traffic.
 *
 * DPDK owns the data NICs and cannot reach the host itself: loopback, the
 * host's kernel interfaces, other processes on the box. Channels bound on or
 * addressed to those go through an ordinary kernel UDP socket instead, set up
 * as Aeron's own kernel transport sets up its sockets
 * (media/aeron_udp_channel_transport.c): non-blocking, SO_RCVBUF/SO_SNDBUF
 * from the channel's params, optionally connected; recvmmsg into the caller's
 * buffers with a zero timeout; sendmmsg. EINTR/EAGAIN/ECONNREFUSED are not
 * errors, as there. None of this runs on a DPDK channel's data path.
 */
#define _GNU_SOURCE /* recvmmsg, sendmmsg */

#include "rusteron_dpdk_internal.h"

#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <sched.h>
#include <time.h>
#include <unistd.h>

#include "util/aeron_error.h"
#include "media/aeron_udp_channel_transport.h"

#define RUSTERON_DPDK_KERNEL_MAX_SEND 64
/* Datagrams one read stamps for a channel asking for media receive
 * timestamps: Aeron's receiver vectors at most this many
 * (AERON_DRIVER_RECEIVER_IO_VECTOR_LENGTH_MAX). */
#define RUSTERON_DPDK_KERNEL_MAX_STAMPED 16

int rusteron_dpdk_kernel_open(
    int *fd_out,
    const struct sockaddr_in *bind_addr,
    const struct sockaddr_in *connect_addr,
    const aeron_udp_channel_transport_params_t *params,
    uint16_t *port_out)
{
    int fd = socket(AF_INET, SOCK_DGRAM | SOCK_NONBLOCK | SOCK_CLOEXEC, 0);
    if (fd < 0)
    {
        AERON_SET_ERR(errno, "%s", "DPDK transport kernel path: socket");
        return -1;
    }
    if (NULL != params && 0 != params->socket_rcvbuf &&
        setsockopt(fd, SOL_SOCKET, SO_RCVBUF, &params->socket_rcvbuf, sizeof(params->socket_rcvbuf)) < 0)
    {
        AERON_SET_ERR(errno, "%s", "DPDK transport kernel path: SO_RCVBUF");
        close(fd);
        return -1;
    }
    if (NULL != params && 0 != params->socket_sndbuf &&
        setsockopt(fd, SOL_SOCKET, SO_SNDBUF, &params->socket_sndbuf, sizeof(params->socket_sndbuf)) < 0)
    {
        AERON_SET_ERR(errno, "%s", "DPDK transport kernel path: SO_SNDBUF");
        close(fd);
        return -1;
    }
    /* A channel asking for media receive timestamps: the kernel stamps each
     * datagram at its receive (CLOCK_REALTIME), as Aeron's kernel transport
     * has it do. Should it refuse, a read stamps with CLOCK_REALTIME itself. */
    const int one = 1;
    if (NULL != params && params->is_media_timestamping)
    {
        (void)setsockopt(fd, SOL_SOCKET, SO_TIMESTAMPNS, &one, sizeof(one));
    }
    if (bind(fd, (const struct sockaddr *)bind_addr, sizeof(*bind_addr)) < 0)
    {
        AERON_SET_ERR(errno, "DPDK transport kernel path: bind to port %u", (unsigned)ntohs(bind_addr->sin_port));
        close(fd);
        return -1;
    }
    if (NULL != connect_addr && connect(fd, (const struct sockaddr *)connect_addr, sizeof(*connect_addr)) < 0)
    {
        AERON_SET_ERR(errno, "%s", "DPDK transport kernel path: connect");
        close(fd);
        return -1;
    }
    struct sockaddr_in bound;
    socklen_t len = sizeof(bound);
    if (getsockname(fd, (struct sockaddr *)&bound, &len) < 0)
    {
        AERON_SET_ERR(errno, "%s", "DPDK transport kernel path: getsockname");
        close(fd);
        return -1;
    }
    *fd_out = fd;
    *port_out = ntohs(bound.sin_port);
    return 0;
}

void rusteron_dpdk_kernel_close(int *fd)
{
    if (*fd >= 0)
    {
        close(*fd);
        *fd = -1;
    }
}

int rusteron_dpdk_kernel_send(
    int fd, const struct sockaddr_storage *address, struct iovec *iov, size_t iov_length, int64_t *bytes_sent)
{
    struct mmsghdr msgs[RUSTERON_DPDK_KERNEL_MAX_SEND];
    size_t sent = 0;
    while (sent < iov_length)
    {
        size_t n = iov_length - sent;
        if (n > RUSTERON_DPDK_KERNEL_MAX_SEND)
        {
            n = RUSTERON_DPDK_KERNEL_MAX_SEND;
        }
        for (size_t i = 0; i < n; i++)
        {
            memset(&msgs[i], 0, sizeof(msgs[i]));
            msgs[i].msg_hdr.msg_name = (void *)address;
            msgs[i].msg_hdr.msg_namelen = NULL != address ? sizeof(struct sockaddr_in) : 0;
            msgs[i].msg_hdr.msg_iov = &iov[sent + i];
            msgs[i].msg_hdr.msg_iovlen = 1;
        }
        int result = sendmmsg(fd, msgs, (unsigned int)n, 0);
        if (result < 0)
        {
            int err = errno;
            if (EINTR == err || EAGAIN == err || EWOULDBLOCK == err || ECONNREFUSED == err)
            {
                break;
            }
            AERON_SET_ERR(err, "%s", "DPDK transport kernel path: sendmmsg");
            return -1;
        }
        for (int i = 0; i < result; i++)
        {
            *bytes_sent += msgs[i].msg_len;
        }
        sent += (size_t)result;
        if ((size_t)result < n)
        {
            break;
        }
    }
    return (int)sent;
}

/* The control buffers a stamped read lent msgvec, taken back: they are its
 * frame's, and Aeron's msgvec outlives it. */
static void rusteron_dpdk_kernel_unstamp(struct mmsghdr *msgvec, size_t vlen, int media_ts)
{
    for (size_t i = 0; media_ts && i < vlen; i++)
    {
        msgvec[i].msg_hdr.msg_control = NULL;
        msgvec[i].msg_hdr.msg_controllen = 0;
    }
}

int rusteron_dpdk_kernel_recv(
    int fd,
    const rusteron_dpdk_client_t *dpdk_client,
    aeron_udp_channel_transport_t *transport,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd)
{
    /* A channel asking for media receive timestamps gets each datagram's
     * from the kernel (SO_TIMESTAMPNS), into control buffers of this call's
     * that are taken off msgvec again before it returns. */
    const int media_ts = 0 != (transport->timestamp_flags & AERON_UDP_CHANNEL_TRANSPORT_MEDIA_RCV_TIMESTAMP);
    union
    {
        char buf[CMSG_SPACE(sizeof(struct timespec))];
        struct cmsghdr align;
    } control[RUSTERON_DPDK_KERNEL_MAX_STAMPED];
    if (media_ts && vlen > RUSTERON_DPDK_KERNEL_MAX_STAMPED)
    {
        vlen = RUSTERON_DPDK_KERNEL_MAX_STAMPED;
    }
    for (size_t i = 0; i < vlen; i++)
    {
        msgvec[i].msg_hdr.msg_namelen = sizeof(struct sockaddr_storage);
        msgvec[i].msg_hdr.msg_control = media_ts ? control[i].buf : NULL;
        msgvec[i].msg_hdr.msg_controllen = media_ts ? sizeof(control[i].buf) : 0;
        msgvec[i].msg_len = 0;
    }
    /* No timeout: with MSG_DONTWAIT it reads until the socket is empty or
     * vlen is reached, so a short read means drained (a zero timeout would
     * return after the first datagram, leaving the rest unflagged). A
     * pending socket error (ICMP port unreachable on a connected socket) is
     * reported before any queued datagram, so it is not "drained": say so. */
    int result = recvmmsg(fd, msgvec, (unsigned int)vlen, MSG_DONTWAIT, NULL);
    struct timespec fallback = { 0, 0 };
    if (result < 0)
    {
        int err = errno;
        rusteron_dpdk_kernel_unstamp(msgvec, vlen, media_ts);
        if (EAGAIN == err || EWOULDBLOCK == err)
        {
            return 0;
        }
        if (EINTR == err || ECONNREFUSED == err)
        {
            return RUSTERON_DPDK_KERNEL_RECV_AGAIN;
        }
        AERON_SET_ERR(err, "%s", "DPDK transport kernel path: recvmmsg");
        return -1;
    }
    for (int i = 0; i < result; i++)
    {
        /* A DPDK transport's kernel socket serves this host only: a remote
         * peer reaching it through a kernel interface would get its replies
         * over DPDK from another address, so it is not heard here; and a
         * connected one hears only its peer, as on the NIC. */
        const struct sockaddr_in *from = msgvec[i].msg_hdr.msg_name;
        if (NULL != dpdk_client &&
            (AF_INET != from->sin_family ||
             !rusteron_dpdk_is_host_local(dpdk_client->runtime, from->sin_addr.s_addr) ||
             (dpdk_client->connected && (from->sin_addr.s_addr != dpdk_client->connected_ip ||
                                         ntohs(from->sin_port) != dpdk_client->connected_port))))
        {
            continue;
        }
        struct timespec *media_timestamp = NULL;
        if (media_ts)
        {
            for (struct cmsghdr *c = CMSG_FIRSTHDR(&msgvec[i].msg_hdr); NULL != c;
                 c = CMSG_NXTHDR(&msgvec[i].msg_hdr, c))
            {
                if (SOL_SOCKET == c->cmsg_level && SCM_TIMESTAMPNS == c->cmsg_type &&
                    CMSG_LEN(sizeof(struct timespec)) == c->cmsg_len)
                {
                    media_timestamp = (struct timespec *)CMSG_DATA(c);
                }
            }
            if (NULL == media_timestamp)
            {
                if (0 == fallback.tv_sec && 0 == fallback.tv_nsec)
                {
                    const uint64_t now = rusteron_dpdk_realtime_ns();
                    fallback.tv_sec = (time_t)(now / 1000000000ULL);
                    fallback.tv_nsec = (long)(now % 1000000000ULL);
                }
                media_timestamp = &fallback;
            }
        }
        recv_func(
            transport->data_paths,
            transport,
            clientd,
            transport->dispatch_clientd,
            transport->destination_clientd,
            msgvec[i].msg_hdr.msg_iov[0].iov_base,
            msgvec[i].msg_len,
            msgvec[i].msg_hdr.msg_name,
            media_timestamp);
        *bytes_rcved += msgvec[i].msg_len;
    }
    rusteron_dpdk_kernel_unstamp(msgvec, vlen, media_ts);
    return result;
}

/* --- The local-I/O thread ---------------------------------------------------
 *
 * Kernel sockets are never polled blindly from the agents' duty cycles. This
 * thread (on the housekeeping CPU) blocks in epoll_wait on every one of them,
 * registered edge-triggered, and only raises a per-socket ready flag. An agent
 * reads the flags (a load each) and, for a ready socket, clears the flag and
 * then calls recvmmsg; a full read leaves it raised. Every datagram arriving
 * after the clear is a new edge that raises it again, so none is missed, and
 * the agent never touches the epoll instance (no epoll_ctl, whose lock the
 * local-I/O thread may hold). So an idle local channel costs the DPDK path
 * nothing, and a busy one is drained every duty cycle until empty. Slots live in a fixed
 * runtime table, never freed, so a late event can only raise a stale flag
 * (a harmless EAGAIN), never touch freed memory.
 *
 * A registered socket costs its senders, though: every datagram runs the
 * epoll's callback and, while this thread sleeps, wakes it, in the sender's
 * own syscall over loopback, even when the agent drains it first and the
 * event itself is dropped. So with adaptive local I/O, when a socket turns
 * hot (its agent reads it every cycle) the agent bumps an epoch, and on its
 * next pass (at most 10 ms on) this thread takes every hot socket off the
 * epoll; the agent keeps reading it every cycle while it is off. Once the
 * agent has cooled, a tick (1 ms while any is off) puts it back, and adding
 * it reports a datagram already waiting, so none is missed. Spin mode never
 * registers a socket: its agents read them all every cycle. */

/* Off the epoll: every watched socket whose agent is hot. Under the lock, so
 * a slot is still its socket's and its fd too. */
static void rusteron_dpdk_ksock_unwatch_hot(rusteron_dpdk_transport_t *native, uint64_t now_ns)
{
    pthread_mutex_lock(&native->port_table_lock);
    for (uint32_t i = 0; i < RUSTERON_DPDK_MAX_KSOCKS; i++)
    {
        rusteron_dpdk_ksock_t *k = &native->ksocks[i];
        if (k->used && k->watched && now_ns < __atomic_load_n(&k->hot_until_ns, __ATOMIC_ACQUIRE) &&
            0 == epoll_ctl(native->local_epfd, EPOLL_CTL_DEL, k->fd, NULL))
        {
            k->watched = 0;
            __atomic_store_n(&k->unwatched, 1, __ATOMIC_RELEASE);
            __atomic_store_n(&native->local_unwatched, native->local_unwatched + 1, __ATOMIC_RELAXED);
        }
    }
    pthread_mutex_unlock(&native->port_table_lock);
}

/* Back on the epoll: every socket off it whose agent has cooled. Added before
 * the agent stops reading every cycle, and adding reports what is waiting. */
static void rusteron_dpdk_ksock_rewatch_cooled(rusteron_dpdk_transport_t *native, uint64_t now_ns)
{
    pthread_mutex_lock(&native->port_table_lock);
    for (uint32_t i = 0; i < RUSTERON_DPDK_MAX_KSOCKS && 0 != native->local_unwatched; i++)
    {
        rusteron_dpdk_ksock_t *k = &native->ksocks[i];
        if (!k->unwatched || now_ns < __atomic_load_n(&k->hot_until_ns, __ATOMIC_ACQUIRE))
        {
            continue;
        }
        struct epoll_event ev = { .events = EPOLLIN | EPOLLET, .data.u32 = i };
        if (0 == epoll_ctl(native->local_epfd, EPOLL_CTL_ADD, k->fd, &ev))
        {
            k->watched = 1;
            __atomic_store_n(&k->unwatched, 0, __ATOMIC_RELEASE);
            __atomic_store_n(&native->local_unwatched, native->local_unwatched - 1, __ATOMIC_RELAXED);
        }
        /* else: still off, its agent still reading every cycle; next tick */
    }
    pthread_mutex_unlock(&native->port_table_lock);
}

static void *rusteron_dpdk_local_io_main(void *arg)
{
    rusteron_dpdk_transport_t *native = arg;
    struct epoll_event events[64];
    const int adaptive = RUSTERON_DPDK_LOCAL_IO_ADAPTIVE == native->local_io_mode && native->local_unwatch;
    uint32_t hot_epoch = 0;
    while (!__atomic_load_n(&native->local_io_stop, __ATOMIC_ACQUIRE))
    {
        const int unwatched = 0 != __atomic_load_n(&native->local_unwatched, __ATOMIC_RELAXED);
        int n = epoll_wait(native->local_epfd, events, 64, unwatched ? 1 : 10);
        const uint64_t now_ns = rusteron_dpdk_clock_now();
        for (int i = 0; i < n; i++)
        {
            const uint32_t slot = events[i].data.u32;
            if (slot < RUSTERON_DPDK_MAX_KSOCKS)
            {
                __atomic_store_n(&native->ksocks[slot].ready, 1, __ATOMIC_RELEASE);
            }
        }
        const uint32_t epoch = __atomic_load_n(&native->local_hot_epoch.value, __ATOMIC_ACQUIRE);
        if (adaptive && epoch != hot_epoch)
        {
            hot_epoch = epoch;
            rusteron_dpdk_ksock_unwatch_hot(native, now_ns);
        }
        if (0 != __atomic_load_n(&native->local_unwatched, __ATOMIC_RELAXED))
        {
            rusteron_dpdk_ksock_rewatch_cooled(native, now_ns);
        }
        rusteron_dpdk_local_io_open_requested(native, now_ns);
    }
    return NULL;
}

/* Claim a free readiness slot; port_table_lock held. -1 when all are used. */
static int rusteron_dpdk_ksock_claim_locked(rusteron_dpdk_transport_t *native)
{
    for (int i = 0; i < RUSTERON_DPDK_MAX_KSOCKS; i++)
    {
        if (!native->ksocks[i].used)
        {
            native->ksocks[i].used = 1;
            __atomic_store_n(&native->ksocks[i].ready, 1, __ATOMIC_RELEASE); /* drain once at the start */
            return i;
        }
    }
    AERON_SET_ERR(EMFILE, "DPDK transport: more than %d host-local sockets", RUSTERON_DPDK_MAX_KSOCKS);
    rusteron_dpdk_set_error("DPDK transport: too many host-local sockets");
    return -1;
}

static void rusteron_dpdk_ksock_release_locked(rusteron_dpdk_transport_t *native, int slot)
{
    rusteron_dpdk_ksock_t *k = &native->ksocks[slot];
    if (k->watched && native->local_epfd >= 0)
    {
        epoll_ctl(native->local_epfd, EPOLL_CTL_DEL, k->fd, NULL);
    }
    if (k->unwatched)
    {
        __atomic_store_n(&native->local_unwatched, native->local_unwatched - 1, __ATOMIC_RELAXED);
    }
    k->used = 0;
    k->watched = 0;
    k->fd = -1;
    __atomic_store_n(&k->unwatched, 0, __ATOMIC_RELEASE);
    __atomic_store_n(&k->hot_until_ns, 0, __ATOMIC_RELEASE);
    __atomic_store_n(&k->ready, 0, __ATOMIC_RELEASE);
}

/* Register a claimed slot's socket with the epoll; port_table_lock held. Spin
 * mode registers nothing: its agents read every socket every cycle. */
static int rusteron_dpdk_ksock_watch_locked(rusteron_dpdk_transport_t *native, int slot, int fd)
{
    rusteron_dpdk_ksock_t *k = &native->ksocks[slot];
    k->fd = fd;
    if (RUSTERON_DPDK_LOCAL_IO_SPIN == native->local_io_mode || native->local_epfd < 0)
    {
        return 0;
    }
    struct epoll_event ev = { .events = EPOLLIN | EPOLLET, .data.u32 = (uint32_t)slot };
    if (epoll_ctl(native->local_epfd, EPOLL_CTL_ADD, fd, &ev) < 0)
    {
        AERON_SET_ERR(errno, "%s", "DPDK transport: epoll_ctl(ADD) for a host-local socket");
        return -1;
    }
    k->watched = 1;
    return 0;
}

/* Open the host-local send sockets agents have asked for (send_local), here
 * so the agents never make the socket/bind syscalls: bound on the
 * destination's own address (the kernel owns it; the NIC's address it does
 * not, and Linux will not send from that to loopback) with the transport's
 * port where free, else a kernel-chosen one. A failure is retried once a
 * second. The client list and the open run under port_table_lock, so a
 * closing transport is never opened for. */
void rusteron_dpdk_local_io_open_requested(rusteron_dpdk_transport_t *native, uint64_t now_ns)
{
    pthread_mutex_lock(&native->port_table_lock);
    for (rusteron_dpdk_client_t *c = native->lazy_clients; NULL != c; c = c->lazy_next)
    {
        const int state = __atomic_load_n(&c->lazy_state, __ATOMIC_ACQUIRE);
        if (!(RUSTERON_DPDK_LAZY_REQUESTED == state || (RUSTERON_DPDK_LAZY_FAILED == state && now_ns >= c->lazy_retry_ns)))
        {
            continue;
        }
        struct sockaddr_in own = {
            .sin_family = AF_INET, .sin_port = htons(c->local_udp_port), .sin_addr.s_addr = c->lazy_dst_ip };
        struct sockaddr_in any_port = { .sin_family = AF_INET, .sin_port = 0, .sin_addr.s_addr = c->lazy_dst_ip };
        int fd = -1;
        uint16_t got = 0;
        int slot = -1;
        if ((rusteron_dpdk_kernel_open(&fd, &own, NULL, &c->params, &got) < 0 &&
             rusteron_dpdk_kernel_open(&fd, &any_port, NULL, &c->params, &got) < 0) ||
            (slot = rusteron_dpdk_ksock_claim_locked(native)) < 0 ||
            rusteron_dpdk_ksock_watch_locked(native, slot, fd) < 0)
        {
            if (slot >= 0)
            {
                rusteron_dpdk_ksock_release_locked(native, slot);
            }
            rusteron_dpdk_kernel_close(&fd);
            c->lazy_retry_ns = now_ns + 1000000000ULL;
            if (RUSTERON_DPDK_LAZY_FAILED != state)
            {
                __atomic_store_n(&c->lazy_state, RUSTERON_DPDK_LAZY_FAILED, __ATOMIC_RELEASE);
            }
            continue;
        }
        c->lazy_fd = fd;
        c->lazy_ksock = slot;
        __atomic_store_n(&c->lazy_state, RUSTERON_DPDK_LAZY_READY, __ATOMIC_RELEASE);
    }
    pthread_mutex_unlock(&native->port_table_lock);
}

int rusteron_dpdk_local_io_start(rusteron_dpdk_transport_t *native, const cpu_set_t *cpus)
{
    native->local_epfd = epoll_create1(EPOLL_CLOEXEC);
    if (native->local_epfd < 0)
    {
        rusteron_dpdk_set_error("cannot create the local-I/O epoll instance");
        return -1;
    }
    native->local_io_stop = 0;
    if (0 != pthread_create(&native->local_io_thread, NULL, rusteron_dpdk_local_io_main, native))
    {
        close(native->local_epfd);
        native->local_epfd = -1;
        rusteron_dpdk_set_error("cannot start the local-I/O thread");
        return -1;
    }
    native->local_io_running = 1;
    pthread_setname_np(native->local_io_thread, "dpdk-local-io");
    if (NULL != cpus)
    {
        const int why = pthread_setaffinity_np(native->local_io_thread, sizeof(*cpus), cpus);
        if (0 != why)
        {
            /* Left unpinned it would run wherever the scheduler puts it, on a
             * trading core included. Fail instead: the start is undone. */
            rusteron_dpdk_local_io_stop(native);
            char names[128];
            char detail[256];
            snprintf(detail, sizeof(detail), "cannot pin the local-I/O thread to CPU %s",
                     rusteron_dpdk_cpu_set_list(cpus, names, sizeof(names)));
            rusteron_dpdk_set_error_code(detail, why);
            return -1;
        }
    }
    return 0;
}

void rusteron_dpdk_local_io_stop(rusteron_dpdk_transport_t *native)
{
    if (native->local_io_running)
    {
        __atomic_store_n(&native->local_io_stop, 1, __ATOMIC_RELEASE);
        pthread_join(native->local_io_thread, NULL);
        native->local_io_running = 0;
    }
    if (native->local_epfd >= 0)
    {
        close(native->local_epfd);
        native->local_epfd = -1;
    }
}

int rusteron_dpdk_ksock_register(rusteron_dpdk_transport_t *native, int fd)
{
    pthread_mutex_lock(&native->port_table_lock);
    int slot = rusteron_dpdk_ksock_claim_locked(native);
    if (slot >= 0 && rusteron_dpdk_ksock_watch_locked(native, slot, fd) < 0)
    {
        rusteron_dpdk_ksock_release_locked(native, slot);
        slot = -1;
    }
    pthread_mutex_unlock(&native->port_table_lock);
    return slot;
}

/* Releases the slot, off the epoll if on it (`fd` is the slot's own). */
void rusteron_dpdk_ksock_unregister(rusteron_dpdk_transport_t *native, int slot, int fd)
{
    (void)fd;
    if (slot < 0)
    {
        return;
    }
    pthread_mutex_lock(&native->port_table_lock);
    rusteron_dpdk_ksock_release_locked(native, slot);
    pthread_mutex_unlock(&native->port_table_lock);
}

void rusteron_dpdk_transport_test_local_watch(const void *transport, int *watched, int *unwatched)
{
    rusteron_dpdk_transport_t *native = (rusteron_dpdk_transport_t *)transport;
    *watched = 0;
    *unwatched = 0;
    pthread_mutex_lock(&native->port_table_lock);
    for (int i = 0; i < RUSTERON_DPDK_MAX_KSOCKS; i++)
    {
        if (native->ksocks[i].used)
        {
            *watched += native->ksocks[i].watched;
            *unwatched += native->ksocks[i].unwatched;
        }
    }
    pthread_mutex_unlock(&native->port_table_lock);
}
