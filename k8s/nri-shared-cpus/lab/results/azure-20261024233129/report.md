# ergon-shared-cpus latency lab

| item | value |
|---|---|
| VM | Standard_D8ps_v6 (8 vCPUs, aarch64), Debian GNU/Linux 13 (trixie), kernel 6.12.113+deb13-cloud-arm64 |
| boot line | `BOOT_IMAGE=/boot/vmlinuz-6.12.113+deb13-cloud-arm64 root=PARTUUID=<redacted> ro initcall_blacklist=arm_pmu_acpi_init transparent_hugepage=madvise nvme_core.io_timeout=240 nvme_core.admin_timeout=240 console=tty0 console=ttyAMA0 earlycon=pl011,0xeffec000 isolcpus=managed_irq,domain,3-7 nohz_full=3-7 rcu_nocbs=3-7 irqaffinity=0` |
| isolated / nohz_full | 3-7 / 3-7 |
| IRQ CPU / shared CPU (scale) / reservedSystemCPUs | 0 / 1 (1-2) / 0-2 |
| node settings | quiet.txt (LAB_QUIET=1) |
| runs | 2 reps x plugin-on, 60 s of spin each, after 5 s of helpers; trace-1: one more plugin-on run under ftrace, after 25 s of helpers |
| helper load | `stress-ng --cpu 1 --vm 1 --vm-bytes 128M` and a loopback UDP reader fed 20000 datagrams/s (a child process of the probe), from the pod's main thread |
| counter | counter_hz=1000000000 (a percentile below one tick reads 0) |
| sampler | /proc/interrupts and /proc/softirqs every 10 ms from CPU 0: what overlaps a max sat within 10 ms of it |
| kernel trace | trace-cpu3.md: every switch, IPI, timer and work item on the traced rep's spin CPU |

## Per arm (median over reps; max of the maxima)

| arm | reps | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs | max in rep |
|---|---|---|---|---|---|---|---|---|
| trace | 1 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 16.25 | 1 |
| plugin-on | 2 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 22.51 | 2 |

## Per run

| arm | rep | cpu | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs | max ended (CLOCK_REALTIME ns) | gaps logged | gaps dropped | involuntary switches | page faults | overlapping the max (counter +growth on its CPU) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace | 1 | 3 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 16.25 | 1792885306582732317 | 6374 | 0 | 1 | 0 | IPI0 Rescheduling interrupts +1, IPI1 Function call interrupts +1 |
| plugin-on | 1 | 3 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 20.94 | 1792885123034749414 | 6374 | 0 | 1 | 0 | IPI0 Rescheduling interrupts +1, IPI1 Function call interrupts +1 |
| plugin-on | 2 | 3 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 22.51 | 1792885213313265248 | 6368 | 0 | 1 | 0 | 10 GICv3 20 Level arch_timer +1, IPI0 Rescheduling interrupts +1, IPI1 Function call interrupts +1 |
