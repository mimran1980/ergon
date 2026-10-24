# ergon-shared-cpus latency lab

| item | value |
|---|---|
| VM | Standard_D8ps_v6 (8 vCPUs, aarch64), Debian GNU/Linux 13 (trixie), kernel 6.12.113+deb13-cloud-arm64 |
| boot line | `BOOT_IMAGE=/boot/vmlinuz-6.12.113+deb13-cloud-arm64 root=PARTUUID=<redacted> ro initcall_blacklist=arm_pmu_acpi_init transparent_hugepage=madvise nvme_core.io_timeout=240 nvme_core.admin_timeout=240 console=tty0 console=ttyAMA0 earlycon=pl011,0xeffec000 isolcpus=managed_irq,domain,2-7 nohz_full=2-7 rcu_nocbs=2-7 irqaffinity=0` |
| isolated / nohz_full | 2-7 / 2-7 |
| IRQ CPU / shared CPU / reservedSystemCPUs | 0 / 1 / 0-1 |
| runs | 5 reps x 2 arms, 60 s of spin each, after 5 s of helpers (the plugin-off rows are left out of this copy: that arm is not a published result) |
| helper load | `stress-ng --cpu 1 --vm 1 --vm-bytes 128M` and a loopback UDP reader fed 20000 datagrams/s, from the pod's main thread |
| counter | counter_hz=1000000000 (a percentile below one tick reads 0) |
| sampler | /proc/interrupts and /proc/softirqs every 10 ms from CPU 0: what overlaps a max sat within 10 ms of it |

## Per arm (median over reps; max of the maxima)

| arm | reps | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs | max in rep |
|---|---|---|---|---|---|---|---|---|
| plugin-on | 5 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 25.34 | 4 |

## Per run

| arm | rep | cpu | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs | max ended (CLOCK_REALTIME ns) | gaps logged | gaps dropped | involuntary switches | overlapping the max (counter +growth on its CPU) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plugin-on | 1 | 2 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 17.40 | 1792877670347178875 | 6318 | 0 | 6 | 10 GICv3 20 Level arch_timer +1, IPI0 Rescheduling interrupts +1, IPI1 Function call interrupts +1 |
| plugin-on | 2 | 2 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 23.47 | 1792877933749166235 | 6368 | 0 | 6 | 10 GICv3 20 Level arch_timer +1, IPI0 Rescheduling interrupts +1, IPI1 Function call interrupts +1 |
| plugin-on | 3 | 2 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 18.44 | 1792878016181161775 | 6362 | 0 | 6 | 10 GICv3 20 Level arch_timer +1, IPI0 Rescheduling interrupts +1, IPI1 Function call interrupts +1 |
| plugin-on | 4 | 2 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 25.34 | 1792878271925148877 | 6369 | 0 | 6 | IPI0 Rescheduling interrupts +1, IPI1 Function call interrupts +1 |
| plugin-on | 5 | 2 | 0.02 | 0.02 | 0.03 | 0.03 | 0.03 | 23.18 | 1792878353333181973 | 6353 | 0 | 6 | IPI0 Rescheduling interrupts +1, IPI1 Function call interrupts +1 |
