/* io_uring_probe: whether this container may call io_uring_setup(2).
 *
 * Prints "io_uring_setup fd=<n>" and exits 0 when the call works, or
 * "io_uring_setup failed: <errno>" and exits 1 when it does not. Under
 * containerd's RuntimeDefault seccomp profile the call is EPERM; the NRI
 * plugin's seccomp grant (seccomp.ergon.io/<container>: "io-uring") widens
 * the profile by one allow rule. A clock probe, not a packet path: Aeron's
 * RTT does not use io_uring.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

struct io_uring_params_local {
    unsigned sq_entries;
    unsigned cq_entries;
    unsigned flags;
    unsigned sq_thread_cpu;
    unsigned sq_thread_idle;
    unsigned features;
    unsigned wq_fd;
    unsigned resv[3];
    unsigned char rest[160];
};

int main(void)
{
    struct io_uring_params_local p;
    int fd;

    memset(&p, 0, sizeof p);
    fd = (int)syscall(__NR_io_uring_setup, 2u, &p);
    if (fd < 0) {
        printf("io_uring_setup failed: %s\n", strerror(errno));
        return 1;
    }
    printf("io_uring_setup fd=%d\n", fd);
    close(fd);
    return 0;
}
