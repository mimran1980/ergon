/* SPDX-License-Identifier: Apache-2.0 */
/*
 * Pin a hot thread by index into ERGON_ISOLATED_CPUS. Leave helpers alone:
 * every thread already starts on the shared CPUs.
 *
 * Build:  cc -O2 -o pin pin.c
 * Run:    in a container that asked for ergon.io/shared-millicpus.
 */
#define _GNU_SOURCE
#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* The i-th number of a list like "1,5"; -1 if there is none. */
static int nth_cpu(const char *list, int i)
{
    if (list == NULL)
        return -1;
    for (const char *p = list; *p != '\0'; i--) {
        char *end;
        long cpu = strtol(p, &end, 10);
        if (end == p)
            return -1;
        if (i == 0)
            return (int)cpu;
        p = (*end == ',') ? end + 1 : "";
    }
    return -1;
}

/* Pin the calling thread to isolated CPU index i. Returns the CPU, or -1. */
int pin_isolated(int i)
{
    int cpu = nth_cpu(getenv("ERGON_ISOLATED_CPUS"), i);
    if (cpu < 0)
        return -1;
    cpu_set_t set;
    CPU_ZERO(&set);
    CPU_SET(cpu, &set);
    return sched_setaffinity(0, sizeof set, &set) == 0 ? cpu : -1;
}

/* Report which CPU the calling thread is on and where its mask is. */
static void report(const char *who)
{
    int now = sched_getcpu();
    cpu_set_t set;
    CPU_ZERO(&set);
    if (sched_getaffinity(0, sizeof set, &set) != 0) {
        perror("sched_getaffinity");
        return;
    }
    printf("%s: running on cpu %d, mask", who, now);
    for (int c = 0; c < CPU_SETSIZE; c++)
        if (CPU_ISSET(c, &set))
            printf(" %d", c);
    printf("\n");
}

int main(void)
{
    const char *iso = getenv("ERGON_ISOLATED_CPUS");
    const char *sh = getenv("ERGON_SHARED_CPUS");
    printf("ERGON_ISOLATED_CPUS=%s\nERGON_SHARED_CPUS=%s\n",
           iso ? iso : "(unset)", sh ? sh : "(unset)");

    /* Every thread starts on the shared CPUs. */
    report("helper (default)");

    /* Pin the hot thread to isolated index 0. */
    int cpu = pin_isolated(0);
    if (cpu < 0) {
        fprintf(stderr, "pin_isolated(0) failed\n");
        return 1;
    }
    printf("pinned the hot thread to isolated index 0 = cpu %d\n", cpu);
    report("hot (pinned)");

    /* Index 1, when the pod asked for more than one CPU. */
    int cpu1 = pin_isolated(1);
    if (cpu1 >= 0) {
        printf("isolated index 1 = cpu %d\n", cpu1);
        report("hot2 (pinned)");
    }
    return 0;
}
