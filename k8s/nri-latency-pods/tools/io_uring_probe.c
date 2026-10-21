/* io_uring_probe: can this process create an io_uring?
 *
 * Prints "fd=<n>" when io_uring_setup(2) succeeds, else "<errno name>:
 * io_uring_setup(2)" (e.g. "EPERM: io_uring_setup(2)") on stdout and exits 1.
 * The kind and EKS tests use it to show RuntimeDefault (containerd 2.x blocks
 * io_uring: PR #9320) against the NRI extension plugin's RuntimeDefault +
 * io_uring allow rule (annotation seccomp.ergon.io/<container>: "io-uring"),
 * and against the shipped Localhost profile. Never Unconfined.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <linux/io_uring.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

int main(void)
{
    struct io_uring_params p;
    memset(&p, 0, sizeof p);
    long fd = syscall(SYS_io_uring_setup, 2, &p);
    if (fd < 0) {
        const char *name = strerrorname_np(errno);
        if (name == NULL)
            name = "ERRNO";
        printf("%s: io_uring_setup(2)\n", name);
        return 1;
    }
    printf("fd=%ld\n", fd);
    return 0;
}
