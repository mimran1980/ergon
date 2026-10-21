# Fair backend comparison

- **Run:** 20261019T175248Z (aws ap-northeast-1 apne1-az4, on-demand priority)
- **Environment:** receiver c8g.2xlarge (Neoverse-V2, aarch64), sender c8g.xlarge, kernel 7.0.0-1014-aws, image Ubuntu 24.04 (Canonical, SSM parameter)
- **Receiver CPUs:** layout own-cores; housekeeping 0, NIC interrupts 1, loop 7, SQPOLL 4, spare 6; boot profile full-pollidle
- **Every figure:** µs, as n (samples) | p50 | p99 | p99.9 | max, per round; the connection count is in each heading.

> **Added 2026-10-20, after the run — read this before ranking any row.**
>
> This file is a frozen artefact. The raw `records.bin` were left on the lab VM
> and the VM is terminated, so it cannot be regenerated; the generator
> (`analyse --fair`) now emits the two accounts below beside every comparison;
> this run predates that, so they were added here by hand.
>
> Two accounts, never one number. `work.late` (in
> [`work-counts.md`](work-counts.md)) counts paced **order slots** filled when
> the next slot was already due. The sender's schedule counts **batches** across
> all the traffic a sender had to send (order connection and feeds together, the
> first message of each due time). A point can report `work.late = 2` and 39,464
> late sender batches at once.
>
> **Six of this run's 144 points sent traffic the run did not plan** — more than
> 1 % of their sender batches began over 100 µs late. All six are in the
> interrupt family; the polling family has none:
>
> | point | batches over 100 µs late |
> |---|---|
> | `f1-irqlocal/rt-uring-spin-n4` | 39,464 (2.49 %) |
> | `f1-irqlocal/rt-uring-spin-n16` | 20,077 (1.27 %) |
> | `f2-irqcore/rt-epoll-spin-n4` | 15,966 (1.01 %) |
> | `f2-irqcore/rt-uring-spin-n4` | 25,728 (1.62 %) |
> | `f2-irqcore/rt-uring-spin-n64` | 19,334 (1.22 %) |
> | `f2-irqlocal/rt-mio-n4` | 21,888 (1.38 %) |
>
> Do not treat those six rows as a like-for-like ranking against the rest. Each
> sits inside its family's unslipped range on p50 and p99, so no published claim
> moves; two endpoints are set by a slipped row and both ranges are in
> [`BENCHMARK.md`](../../BENCHMARK.md).
>
> Separately: every `md-mio-c1` point shows a sender `late >100 µs` **max** of
> 36,059–40,180 µs on 0.07 % of batches (next largest anywhere in the run:
> 197 µs). Confined to `mio` at one connection across every flag combination.
> No latency sample saw it, so the `md-mio-c1` latency figures stand. Its cause
> is **not traced** ([`BENCHMARK.md`](../../BENCHMARK.md#open), Open).

**Settings every backend shares** (held, and checked before and after every point):

- net.core.busy_poll 0, net.core.busy_read 0 (no global busy polling: the interrupt-driven backends are purely interrupt driven; recv-busy and uring-sqpoll set their own per socket or ring)
- NIC interrupt moderation off where the NIC has it: adaptive-rx off, rx-usecs 0, RX CQE coalescing off (rx-cqe-frames 1); a NIC without the setting is recorded as such
- napi_defer_hard_irqs 0, gro_flush_timeout 0 on every NIC
- every NIC queue interrupt on the IRQ core (irqcore), or on the loop's CPU (irqlocal); every other movable interrupt on housekeeping; irqbalance off
- idle CPUs poll instead of halting where no SMT sibling would spin (boot profile full-pollidle: x86 idle=poll, arm64 nohlt), so the IRQ core never wakes from sleep for an interrupt; else halt (recorded)
- isolcpus, nohz_full and rcu_nocbs on every CPU but housekeeping; routes with quickack 1, rto_min 5ms
- the sender: the same settings, its NIC interrupts on its housekeeping CPU, its server loop on the other CPU (the round-trip and HTTP servers busy poll their sockets; the market-data sender only sends)
- kernel trace of the loop CPU (and the SQPOLL CPU) for the cause of each stall; in irqlocal without interrupt and softirq events, which there fire on every packet of the interrupt-driven backends only (/proc/interrupts still counts them)

**As found at the start:** `receiver.boot-profile=full-pollidle`, `receiver.idle=poll-nohlt`, `receiver.sysctl.net.core.busy_poll=0`, `receiver.sysctl.net.core.busy_read=0`, `receiver.nic.ens34.driver=ena`, `receiver.nic.ens34.adaptive-rx=off`, `receiver.nic.ens34.rx-usecs=0`, `receiver.nic.ens34.rx-frames=n/a`, `receiver.nic.ens34.rx-cqe-frames=-`, `receiver.nic.ens34.napi_defer_hard_irqs=0`, `receiver.nic.ens34.gro_flush_timeout=0`, `sender.boot-profile=full-pollidle`, `sender.idle=poll-nohlt`, `sender.sysctl.net.core.busy_poll=0`, `sender.sysctl.net.core.busy_read=0`, `sender.nic.ens34.driver=ena`, `sender.nic.ens34.adaptive-rx=off`, `sender.nic.ens34.rx-usecs=0`, `sender.nic.ens34.rx-frames=n/a`, `sender.nic.ens34.rx-cqe-frames=-`, `sender.nic.ens34.napi_defer_hard_irqs=0`, `sender.nic.ens34.gro_flush_timeout=0`

**What each backend sets for itself:**

- `epoll-spin`: interrupt: ergon-io: epoll_wait(0) in a spin loop, SO_BUSY_POLL 0 on its sockets: purely interrupt driven
- `mio`: interrupt: mio + rustls (the usual Rust way): Poll::poll(0) (epoll) in a spin loop, no busy polling: purely interrupt driven
- `uring-spin`: interrupt: ergon-io: io_uring submitted and reaped without waiting, no NAPI registration: interrupt driven
- `recv-busy`: polling: ergon-io: a non-blocking recvmsg on every socket every poll, SO_BUSY_POLL 1 µs: each empty read polls the NIC queue once
- `recv-busy-pq`: polling: ergon-io: recv-busy reading one socket per NIC queue, then epoll_wait(0) for the rest (Config::per_queue)
- `uring-sqpoll`: polling: ergon-io: io_uring with its SQPOLL kernel thread on its own core (sqpoll_cpu), NAPI busy poll 1 µs registered for its sockets' queues

## Market data, sender → decoded: every backend against the others

The whole path: the sender's send time to decoded, NIC, interrupt and loop included. Two hosts' clocks, so each value includes their offset (the bound column: chrony's estimate); the backends of one table ran minutes apart, so compare them with each other, not with other tables.

### irqcore, 1 connection

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950177 | 28.25 | 51.46 | 57.07 | 114.12 | 203.09 |
| `epoll-spin` | interrupt | 2 | 2950172 | 43.72 | 67.47 | 74.38 | 123.29 | 205.39 |
| `mio` | interrupt | 1 | 2950149 | 26.19 | 51.60 | 63.32 | 516.11 | 208.28 |
| `mio` | interrupt | 2 | 2950149 | 48.69 | 73.58 | 84.49 | 173.77 | 179.39 |
| `uring-spin` | interrupt | 1 | 2950229 | 25.93 | 51.52 | 64.66 | 416.19 | 233.57 |
| `uring-spin` | interrupt | 2 | 2950175 | 46.08 | 69.81 | 77.28 | 159.73 | 181.44 |
| `recv-busy` | polling | 1 | 2950172 | 20.53 | 45.46 | 52.21 | 126.08 | 208.45 |
| `recv-busy` | polling | 2 | 2950226 | 45.88 | 69.61 | 76.90 | 137.38 | 207.00 |
| `recv-busy-pq` | polling | 1 | 2950225 | 26.60 | 50.82 | 58.78 | 1787.92 | 209.19 |
| `recv-busy-pq` | polling | 2 | 2950173 | 46.43 | 70.20 | 77.39 | 165.36 | 167.98 |
| `uring-sqpoll` | polling | 1 | 2950172 | 27.71 | 50.94 | 57.42 | 96.68 | 189.79 |
| `uring-sqpoll` | polling | 2 | 2950232 | 45.50 | 69.13 | 76.23 | 125.16 | 182.88 |

### irqcore, 4 connections

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950228 | 27.09 | 57.95 | 68.61 | 143.15 | 201.94 |
| `epoll-spin` | interrupt | 2 | 2950190 | 30.03 | 61.44 | 73.66 | 151.03 | 187.37 |
| `mio` | interrupt | 1 | 2950220 | 30.12 | 63.49 | 74.34 | 147.25 | 194.18 |
| `mio` | interrupt | 2 | 2950225 | 40.52 | 72.41 | 85.11 | 151.14 | 199.44 |
| `uring-spin` | interrupt | 1 | 2950222 | 28.56 | 58.12 | 68.44 | 128.70 | 199.38 |
| `uring-spin` | interrupt | 2 | 2950169 | 39.65 | 70.52 | 82.82 | 143.88 | 200.12 |
| `recv-busy` | polling | 1 | 2950232 | 22.51 | 51.90 | 62.10 | 152.50 | 192.97 |
| `recv-busy` | polling | 2 | 2950184 | 39.37 | 69.74 | 80.77 | 159.58 | 194.01 |
| `recv-busy-pq` | polling | 1 | 2950233 | 20.66 | 50.33 | 60.56 | 146.24 | 190.61 |
| `recv-busy-pq` | polling | 2 | 2950231 | 40.55 | 72.26 | 83.95 | 167.54 | 192.82 |
| `uring-sqpoll` | polling | 1 | 2950216 | 25.15 | 55.64 | 66.01 | 112.18 | 205.32 |
| `uring-sqpoll` | polling | 2 | 2950221 | 39.89 | 72.20 | 83.69 | 156.35 | 198.28 |

### irqcore, 16 connections

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950231 | 20.48 | 60.74 | 73.99 | 131.35 | 206.45 |
| `epoll-spin` | interrupt | 2 | 2950239 | 28.51 | 70.89 | 87.64 | 177.53 | 181.48 |
| `mio` | interrupt | 1 | 2950236 | 21.62 | 62.26 | 75.95 | 132.51 | 193.34 |
| `mio` | interrupt | 2 | 2950237 | 29.70 | 74.09 | 90.71 | 193.56 | 184.53 |
| `uring-spin` | interrupt | 1 | 2950214 | 24.45 | 67.56 | 81.17 | 152.09 | 218.98 |
| `uring-spin` | interrupt | 2 | 2950169 | 29.26 | 71.94 | 88.56 | 157.75 | 181.35 |
| `recv-busy` | polling | 1 | 2950243 | 36.22 | 76.74 | 90.20 | 712.32 | 232.17 |
| `recv-busy` | polling | 2 | 2950242 | 32.47 | 75.02 | 90.32 | 159.54 | 191.29 |
| `recv-busy-pq` | polling | 1 | 2950234 | 34.26 | 77.08 | 91.04 | 139.40 | 227.71 |
| `recv-busy-pq` | polling | 2 | 2950242 | 30.33 | 74.09 | 90.30 | 142.04 | 200.62 |
| `uring-sqpoll` | polling | 1 | 2950159 | 31.84 | 71.62 | 84.88 | 130.68 | 184.69 |
| `uring-sqpoll` | polling | 2 | 2950168 | 27.53 | 67.95 | 82.57 | 242.64 | 205.98 |

### irqcore, 64 connections

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950231 | 29.83 | 74.53 | 89.73 | 210.57 | 179.97 |
| `epoll-spin` | interrupt | 2 | 2950241 | 31.38 | 78.64 | 96.44 | 167.01 | 203.48 |
| `mio` | interrupt | 1 | 2950232 | 28.19 | 72.56 | 88.32 | 139.13 | 196.60 |
| `mio` | interrupt | 2 | 2950241 | 33.55 | 82.71 | 101.29 | 163.76 | 203.56 |
| `uring-spin` | interrupt | 1 | 2950161 | 19.90 | 64.48 | 79.52 | 154.88 | 194.56 |
| `uring-spin` | interrupt | 2 | 2950163 | 32.61 | 80.06 | 97.74 | 137.09 | 182.49 |
| `recv-busy` | polling | 1 | 2950317 | 48.06 | 103.32 | 117.59 | 173.75 | 190.88 |
| `recv-busy` | polling | 2 | 2950262 | 54.05 | 110.44 | 125.94 | 201.51 | 211.15 |
| `recv-busy-pq` | polling | 1 | 2950236 | 26.32 | 73.81 | 88.85 | 172.95 | 193.13 |
| `recv-busy-pq` | polling | 2 | 2950245 | 35.05 | 84.15 | 101.48 | 168.27 | 205.01 |
| `uring-sqpoll` | polling | 1 | 2950161 | 23.28 | 65.57 | 79.45 | 125.00 | 184.35 |
| `uring-sqpoll` | polling | 2 | 2950166 | 29.23 | 73.04 | 89.26 | 193.19 | 224.71 |

### irqcore, 256 connections

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950230 | 24.60 | 70.21 | 85.99 | 139.93 | 196.12 |
| `epoll-spin` | interrupt | 2 | 2950240 | 31.26 | 80.70 | 99.20 | 185.16 | 167.14 |
| `mio` | interrupt | 1 | 2950229 | 25.12 | 73.04 | 89.56 | 164.70 | 196.62 |
| `mio` | interrupt | 2 | 2950226 | 30.92 | 79.49 | 99.11 | 158.16 | 170.68 |
| `uring-spin` | interrupt | 1 | 2950157 | 28.85 | 74.77 | 90.49 | 155.00 | 192.59 |
| `uring-spin` | interrupt | 2 | 2950162 | 30.64 | 79.52 | 98.10 | 141.52 | 180.19 |
| `recv-busy` | polling | 1 | 2950312 | 112.47 | 213.40 | 231.95 | 362.70 | 204.93 |
| `recv-busy` | polling | 2 | 2950682 | 117.92 | 219.72 | 239.73 | 285.05 | 196.89 |
| `recv-busy-pq` | polling | 1 | 2950233 | 34.07 | 84.20 | 111.50 | 197.03 | 207.46 |
| `recv-busy-pq` | polling | 2 | 2950243 | 37.42 | 92.11 | 125.59 | 214.12 | 200.63 |
| `uring-sqpoll` | polling | 1 | 2950216 | 29.47 | 72.99 | 87.53 | 155.74 | 214.61 |
| `uring-sqpoll` | polling | 2 | 2950157 | 31.42 | 76.52 | 93.62 | 172.99 | 201.45 |

### irqlocal, 1 connection

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950222 | 34.07 | 57.20 | 65.81 | 320.61 | 183.55 |
| `epoll-spin` | interrupt | 2 | 2950171 | 33.11 | 57.20 | 63.95 | 117.51 | 180.03 |
| `mio` | interrupt | 1 | 2950120 | 42.89 | 73.38 | 84.28 | 154.90 | 205.06 |
| `mio` | interrupt | 2 | 2950101 | 37.52 | 69.69 | 82.27 | 175.40 | 185.32 |
| `uring-spin` | interrupt | 1 | 2950224 | 36.51 | 59.68 | 66.63 | 148.88 | 190.75 |
| `uring-spin` | interrupt | 2 | 2950225 | 38.98 | 63.46 | 75.21 | 290.49 | 199.62 |

### irqlocal, 4 connections

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950160 | 36.81 | 81.09 | 98.42 | 186.23 | 207.31 |
| `epoll-spin` | interrupt | 2 | 2950159 | 31.49 | 76.41 | 94.35 | 129.24 | 189.68 |
| `mio` | interrupt | 1 | 2950145 | 40.85 | 99.50 | 115.88 | 362.24 | 207.00 |
| `mio` | interrupt | 2 | 2950197 | 35.19 | 89.69 | 106.50 | 148.41 | 197.47 |
| `uring-spin` | interrupt | 1 | 2950142 | 37.93 | 80.83 | 98.00 | 162.79 | 196.84 |
| `uring-spin` | interrupt | 2 | 2950141 | 33.15 | 80.49 | 98.73 | 161.93 | 183.75 |

### irqlocal, 16 connections

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950222 | 37.57 | 105.16 | 122.67 | 172.59 | 230.71 |
| `epoll-spin` | interrupt | 2 | 2950245 | 35.15 | 103.48 | 121.55 | 189.16 | 194.90 |
| `mio` | interrupt | 1 | 2950207 | 41.83 | 115.76 | 133.66 | 254.92 | 218.26 |
| `mio` | interrupt | 2 | 2950237 | 38.34 | 113.08 | 131.40 | 205.50 | 218.01 |
| `uring-spin` | interrupt | 1 | 2950155 | 38.91 | 109.01 | 126.81 | 185.16 | 191.34 |
| `uring-spin` | interrupt | 2 | 2950156 | 33.65 | 104.61 | 123.19 | 172.45 | 192.87 |

### irqlocal, 64 connections

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950242 | 40.68 | 115.40 | 133.43 | 178.64 | 195.82 |
| `epoll-spin` | interrupt | 2 | 2950235 | 32.17 | 107.19 | 125.55 | 217.17 | 196.18 |
| `mio` | interrupt | 1 | 2950236 | 45.53 | 123.91 | 142.47 | 241.11 | 205.37 |
| `mio` | interrupt | 2 | 2950247 | 38.35 | 118.51 | 137.32 | 182.65 | 209.98 |
| `uring-spin` | interrupt | 1 | 2950152 | 42.26 | 121.02 | 139.63 | 269.81 | 197.53 |
| `uring-spin` | interrupt | 2 | 2950150 | 36.26 | 115.71 | 135.52 | 186.64 | 213.38 |

### irqlocal, 256 connections

| backend | family | round | n | p50 | p99 | p99.9 | max | clock bound ± |
|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 2950245 | 46.29 | 125.32 | 146.41 | 195.90 | 172.42 |
| `epoll-spin` | interrupt | 2 | 2950248 | 39.34 | 118.77 | 138.45 | 191.58 | 189.09 |
| `mio` | interrupt | 1 | 2950233 | 51.97 | 133.81 | 152.45 | 213.76 | 178.38 |
| `mio` | interrupt | 2 | 2950243 | 42.13 | 124.38 | 144.75 | 206.13 | 196.91 |
| `uring-spin` | interrupt | 1 | 2950154 | 49.50 | 136.92 | 157.43 | 218.92 | 168.98 |
| `uring-spin` | interrupt | 2 | 2950141 | 37.58 | 124.67 | 145.88 | 189.22 | 177.90 |

## Market data, kernel arrival → decoded: within a family only

One clock (the receiver's), so exact, but not comparable across families: for an interrupt-driven backend the kernel stamps arrival after the interrupt, so this leaves its interrupt delay out. mio reads without kernel timestamps and has no row here.

### polling backends, irqcore, 1 connection

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `recv-busy` | 1 | 2950172 | 2.17 | 6.16 | 9.12 | 40.92 |
| `recv-busy` | 2 | 2950226 | 2.21 | 6.30 | 8.52 | 25.32 |
| `recv-busy-pq` | 1 | 2950225 | 2.55 | 6.84 | 9.19 | 74.85 |
| `recv-busy-pq` | 2 | 2950173 | 2.56 | 6.76 | 9.29 | 48.70 |
| `uring-sqpoll` | 1 | 2950172 | 2.70 | 6.63 | 9.15 | 71.02 |
| `uring-sqpoll` | 2 | 2950232 | 2.70 | 6.86 | 9.97 | 32.52 |

### polling backends, irqcore, 4 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `recv-busy` | 1 | 2950232 | 2.68 | 8.39 | 13.56 | 134.47 |
| `recv-busy` | 2 | 2950184 | 2.79 | 9.18 | 15.81 | 119.13 |
| `recv-busy-pq` | 1 | 2950233 | 3.08 | 10.04 | 15.21 | 101.61 |
| `recv-busy-pq` | 2 | 2950231 | 3.02 | 9.92 | 16.36 | 117.24 |
| `uring-sqpoll` | 1 | 2950216 | 2.98 | 9.54 | 17.38 | 89.10 |
| `uring-sqpoll` | 2 | 2950221 | 3.04 | 9.54 | 18.49 | 91.30 |

### polling backends, irqcore, 16 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `recv-busy` | 1 | 2950243 | 6.44 | 19.99 | 27.87 | 140.52 |
| `recv-busy` | 2 | 2950242 | 6.56 | 20.41 | 29.68 | 106.69 |
| `recv-busy-pq` | 1 | 2950234 | 5.35 | 16.19 | 22.66 | 49.10 |
| `recv-busy-pq` | 2 | 2950242 | 5.34 | 16.77 | 24.30 | 113.37 |
| `uring-sqpoll` | 1 | 2950159 | 3.67 | 12.82 | 23.61 | 90.67 |
| `uring-sqpoll` | 2 | 2950168 | 3.52 | 13.32 | 25.82 | 72.96 |

### polling backends, irqcore, 64 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `recv-busy` | 1 | 2950317 | 20.64 | 55.92 | 67.37 | 141.15 |
| `recv-busy` | 2 | 2950262 | 21.05 | 57.13 | 69.75 | 175.68 |
| `recv-busy-pq` | 1 | 2950236 | 6.27 | 18.74 | 27.55 | 151.00 |
| `recv-busy-pq` | 2 | 2950245 | 6.26 | 19.29 | 29.84 | 108.76 |
| `uring-sqpoll` | 1 | 2950161 | 3.79 | 13.88 | 25.22 | 104.99 |
| `uring-sqpoll` | 2 | 2950166 | 3.84 | 14.69 | 28.45 | 77.04 |

### polling backends, irqcore, 256 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `recv-busy` | 1 | 2950312 | 78.80 | 170.97 | 186.34 | 244.08 |
| `recv-busy` | 2 | 2950682 | 78.77 | 171.47 | 187.62 | 251.81 |
| `recv-busy-pq` | 1 | 2950233 | 6.81 | 21.41 | 75.58 | 140.88 |
| `recv-busy-pq` | 2 | 2950243 | 7.11 | 22.89 | 84.39 | 131.34 |
| `uring-sqpoll` | 1 | 2950216 | 4.08 | 14.89 | 27.07 | 99.08 |
| `uring-sqpoll` | 2 | 2950157 | 4.17 | 15.87 | 29.93 | 112.94 |

### interrupt backends, irqcore, 1 connection

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950177 | 2.49 | 6.48 | 9.33 | 27.79 |
| `epoll-spin` | 2 | 2950172 | 2.50 | 6.81 | 9.59 | 49.80 |
| `uring-spin` | 1 | 2950229 | 2.55 | 6.89 | 9.06 | 60.03 |
| `uring-spin` | 2 | 2950175 | 2.60 | 7.14 | 9.82 | 25.80 |

### interrupt backends, irqcore, 4 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950228 | 2.54 | 6.80 | 9.85 | 88.82 |
| `epoll-spin` | 2 | 2950190 | 2.54 | 6.89 | 9.31 | 78.12 |
| `uring-spin` | 1 | 2950222 | 2.59 | 7.48 | 10.28 | 81.79 |
| `uring-spin` | 2 | 2950169 | 2.64 | 7.40 | 10.37 | 39.38 |

### interrupt backends, irqcore, 16 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950231 | 2.57 | 7.72 | 10.23 | 75.00 |
| `epoll-spin` | 2 | 2950239 | 2.63 | 8.38 | 11.37 | 71.25 |
| `uring-spin` | 1 | 2950214 | 2.70 | 9.75 | 13.30 | 77.74 |
| `uring-spin` | 2 | 2950169 | 2.77 | 9.19 | 12.29 | 118.87 |

### interrupt backends, irqcore, 64 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950231 | 2.67 | 8.31 | 10.89 | 145.26 |
| `epoll-spin` | 2 | 2950241 | 2.65 | 8.45 | 11.27 | 98.41 |
| `uring-spin` | 1 | 2950161 | 2.73 | 9.14 | 11.93 | 141.68 |
| `uring-spin` | 2 | 2950163 | 2.78 | 9.34 | 12.31 | 30.51 |

### interrupt backends, irqcore, 256 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950230 | 2.84 | 8.59 | 11.24 | 120.16 |
| `epoll-spin` | 2 | 2950240 | 2.98 | 9.00 | 11.97 | 127.74 |
| `uring-spin` | 1 | 2950157 | 2.90 | 9.73 | 12.78 | 131.39 |
| `uring-spin` | 2 | 2950162 | 3.11 | 10.26 | 13.52 | 84.46 |

### interrupt backends, irqlocal, 1 connection

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950222 | 2.85 | 10.15 | 14.34 | 30.92 |
| `epoll-spin` | 2 | 2950171 | 2.86 | 11.73 | 19.77 | 33.52 |
| `uring-spin` | 1 | 2950224 | 2.72 | 10.97 | 18.06 | 40.78 |
| `uring-spin` | 2 | 2950225 | 2.77 | 10.77 | 16.33 | 115.76 |

### interrupt backends, irqlocal, 4 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950160 | 3.52 | 22.84 | 38.28 | 78.39 |
| `epoll-spin` | 2 | 2950159 | 3.58 | 25.72 | 41.98 | 77.09 |
| `uring-spin` | 1 | 2950142 | 3.32 | 25.70 | 42.21 | 80.10 |
| `uring-spin` | 2 | 2950141 | 3.50 | 30.17 | 47.98 | 95.88 |

### interrupt backends, irqlocal, 16 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950222 | 4.56 | 48.39 | 66.74 | 122.89 |
| `epoll-spin` | 2 | 2950245 | 4.68 | 49.65 | 67.89 | 110.20 |
| `uring-spin` | 1 | 2950155 | 4.79 | 54.70 | 72.89 | 123.04 |
| `uring-spin` | 2 | 2950156 | 4.80 | 56.09 | 74.96 | 130.30 |

### interrupt backends, irqlocal, 64 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950242 | 5.39 | 59.36 | 78.17 | 116.56 |
| `epoll-spin` | 2 | 2950235 | 5.27 | 59.16 | 78.51 | 125.22 |
| `uring-spin` | 1 | 2950152 | 5.85 | 67.45 | 86.67 | 136.71 |
| `uring-spin` | 2 | 2950150 | 5.97 | 68.68 | 88.28 | 142.31 |

### interrupt backends, irqlocal, 256 connections

| backend | round | n | p50 | p99 | p99.9 | max |
|---|---|---|---|---|---|---|
| `epoll-spin` | 1 | 2950245 | 7.93 | 69.63 | 89.71 | 139.48 |
| `epoll-spin` | 2 | 2950248 | 7.93 | 69.98 | 89.43 | 154.53 |
| `uring-spin` | 1 | 2950154 | 9.99 | 83.70 | 104.24 | 158.74 |
| `uring-spin` | 2 | 2950141 | 9.77 | 82.57 | 103.76 | 158.87 |

## The WebSocket order round trip with feed connections on the same loop

One order connection, echoed by the server, while N-1 more connections on the same loop carry the market data (decoded, not timed). One clock (the receiver's): send to echo decoded. "From due" times each paced round trip from its slot on the schedule, so it includes any wait behind a stall.

### irqcore, N = 1 (the order connection alone), 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589999 | 39.16 | 47.02 | 54.75 | 130.71 | 47.27 | 130.80 |
| `epoll-spin` | interrupt | 2 | 589999 | 38.88 | 47.42 | 55.53 | 145.15 | 47.66 | 146.31 |
| `mio` | interrupt | 1 | 589999 | 39.09 | 46.97 | 55.04 | 107.72 | 47.17 | 136.84 |
| `mio` | interrupt | 2 | 589999 | 39.33 | 48.37 | 57.41 | 132.85 | 48.58 | 158.74 |
| `uring-spin` | interrupt | 1 | 589999 | 38.83 | 47.82 | 55.70 | 108.63 | 47.89 | 145.47 |
| `uring-spin` | interrupt | 2 | 589999 | 38.78 | 47.60 | 56.09 | 143.70 | 47.68 | 143.79 |
| `recv-busy` | polling | 1 | 589999 | 36.01 | 43.90 | 50.79 | 148.12 | 44.25 | 148.53 |
| `recv-busy` | polling | 2 | 589999 | 36.22 | 45.95 | 64.94 | 283.50 | 46.33 | 402.21 |
| `recv-busy-pq` | polling | 1 | 589999 | 36.92 | 44.70 | 51.79 | 124.54 | 45.20 | 124.59 |
| `recv-busy-pq` | polling | 2 | 589999 | 36.79 | 45.33 | 52.70 | 137.85 | 45.94 | 148.01 |
| `uring-sqpoll` | polling | 1 | 589999 | 36.48 | 45.37 | 61.01 | 261.63 | 45.47 | 479.19 |
| `uring-sqpoll` | polling | 2 | 589999 | 37.04 | 45.52 | 52.46 | 121.59 | 45.59 | 121.67 |

### irqcore, N = 4 (1 order connection + 3 feed connections), 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589997 | 39.53 | 50.27 | 63.35 | 300.81 | 50.66 | 407.00 |
| `epoll-spin` | interrupt | 2 | 589997 | 39.61 | 50.81 | 60.26 | 135.64 | 51.20 | 135.70 |
| `mio` | interrupt | 1 | 589996 | 39.90 | 49.45 | 57.66 | 181.97 | 49.88 | 182.22 |
| `mio` | interrupt | 2 | 589996 | 39.58 | 51.20 | 59.98 | 214.91 | 51.75 | 215.20 |
| `uring-spin` | interrupt | 1 | 589997 | 39.41 | 49.15 | 56.95 | 149.58 | 49.32 | 149.62 |
| `uring-spin` | interrupt | 2 | 589997 | 39.44 | 50.72 | 59.44 | 162.78 | 50.99 | 162.87 |
| `recv-busy` | polling | 1 | 589998 | 39.91 | 49.71 | 57.41 | 129.13 | 51.64 | 180.61 |
| `recv-busy` | polling | 2 | 589997 | 39.68 | 49.68 | 58.98 | 152.41 | 51.94 | 153.22 |
| `recv-busy-pq` | polling | 1 | 589997 | 40.49 | 50.83 | 60.05 | 127.32 | 53.31 | 186.30 |
| `recv-busy-pq` | polling | 2 | 589997 | 39.19 | 50.09 | 59.01 | 141.19 | 52.16 | 151.66 |
| `uring-sqpoll` | polling | 1 | 589997 | 39.68 | 50.85 | 60.23 | 165.46 | 50.95 | 165.51 |
| `uring-sqpoll` | polling | 2 | 589997 | 38.74 | 48.67 | 57.44 | 118.75 | 48.78 | 118.83 |

### irqcore, N = 16 (1 order connection + 15 feed connections), 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589997 | 39.59 | 50.02 | 61.13 | 193.41 | 50.64 | 221.61 |
| `epoll-spin` | interrupt | 2 | 589997 | 39.97 | 51.63 | 63.45 | 157.56 | 52.13 | 175.99 |
| `mio` | interrupt | 1 | 589996 | 39.70 | 49.91 | 59.92 | 167.75 | 50.52 | 167.87 |
| `mio` | interrupt | 2 | 589995 | 40.14 | 51.51 | 62.23 | 155.63 | 52.12 | 155.71 |
| `uring-spin` | interrupt | 1 | 589997 | 39.27 | 49.80 | 59.29 | 139.69 | 50.20 | 139.79 |
| `uring-spin` | interrupt | 2 | 589997 | 39.80 | 51.83 | 61.92 | 140.64 | 52.12 | 140.69 |
| `recv-busy` | polling | 1 | 590001 | 48.98 | 66.33 | 77.15 | 135.92 | 75.03 | 218.93 |
| `recv-busy` | polling | 2 | 590002 | 49.63 | 69.16 | 82.41 | 155.99 | 78.56 | 179.49 |
| `recv-busy-pq` | polling | 1 | 589997 | 44.58 | 59.94 | 71.33 | 179.16 | 64.32 | 182.09 |
| `recv-busy-pq` | polling | 2 | 589999 | 44.33 | 60.41 | 74.30 | 164.01 | 65.89 | 182.78 |
| `uring-sqpoll` | polling | 1 | 589997 | 38.98 | 50.15 | 60.31 | 127.99 | 50.25 | 165.86 |
| `uring-sqpoll` | polling | 2 | 589997 | 39.75 | 51.41 | 61.84 | 153.14 | 51.51 | 153.20 |

### irqcore, N = 64 (1 order connection + 63 feed connections), 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589996 | 40.17 | 51.93 | 69.85 | 145.90 | 52.40 | 146.96 |
| `epoll-spin` | interrupt | 2 | 589996 | 40.42 | 53.85 | 70.91 | 143.12 | 54.41 | 167.21 |
| `mio` | interrupt | 1 | 589995 | 40.08 | 52.32 | 71.19 | 178.02 | 52.93 | 178.32 |
| `mio` | interrupt | 2 | 589995 | 39.93 | 53.10 | 71.54 | 425.45 | 53.64 | 802.26 |
| `uring-spin` | interrupt | 1 | 589996 | 39.88 | 51.86 | 70.02 | 185.31 | 52.28 | 185.40 |
| `uring-spin` | interrupt | 2 | 589996 | 39.96 | 53.81 | 70.66 | 153.34 | 54.19 | 153.42 |
| `recv-busy` | polling | 1 | 493355 | 78.14 | 118.66 | 142.11 | 197.40 | 9732739.14 | 9829155.47 |
| `recv-busy` | polling | 2 | 493595 | 78.29 | 119.47 | 146.55 | 265.44 | 9706514.34 | 9802987.93 |
| `recv-busy-pq` | polling | 1 | 589998 | 44.26 | 61.69 | 79.49 | 133.54 | 67.44 | 189.74 |
| `recv-busy-pq` | polling | 2 | 589997 | 44.73 | 62.22 | 81.01 | 144.64 | 67.67 | 182.37 |
| `uring-sqpoll` | polling | 1 | 589997 | 40.04 | 53.69 | 71.71 | 110.48 | 53.84 | 125.70 |
| `uring-sqpoll` | polling | 2 | 589996 | 40.07 | 54.76 | 73.35 | 130.53 | 54.89 | 130.66 |

### irqlocal, N = 1 (the order connection alone), 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589999 | 38.52 | 46.49 | 54.26 | 161.30 | 46.73 | 161.49 |
| `epoll-spin` | interrupt | 2 | 589999 | 38.87 | 46.97 | 54.56 | 152.31 | 47.19 | 165.32 |
| `mio` | interrupt | 1 | 589999 | 38.97 | 47.01 | 54.95 | 159.38 | 47.19 | 159.44 |
| `mio` | interrupt | 2 | 589999 | 38.88 | 47.17 | 55.16 | 148.73 | 47.36 | 149.04 |
| `uring-spin` | interrupt | 1 | 589999 | 37.93 | 45.87 | 53.45 | 134.98 | 45.96 | 135.03 |
| `uring-spin` | interrupt | 2 | 589999 | 38.65 | 46.80 | 54.77 | 151.31 | 46.89 | 159.35 |

### irqlocal, N = 4 (1 order connection + 3 feed connections), 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589996 | 40.82 | 72.67 | 90.74 | 218.84 | 76.71 | 226.66 |
| `epoll-spin` | interrupt | 2 | 589997 | 40.67 | 54.92 | 68.88 | 128.56 | 57.73 | 175.14 |
| `mio` | interrupt | 1 | 589995 | 40.15 | 58.12 | 77.65 | 139.98 | 60.61 | 160.96 |
| `mio` | interrupt | 2 | 589995 | 40.22 | 66.73 | 95.99 | 145.84 | 75.48 | 170.88 |
| `uring-spin` | interrupt | 1 | 589996 | 39.34 | 56.79 | 77.65 | 151.33 | 59.73 | 204.15 |
| `uring-spin` | interrupt | 2 | 589996 | 39.55 | 52.31 | 65.82 | 151.79 | 54.78 | 187.34 |

### irqlocal, N = 16 (1 order connection + 15 feed connections), 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589996 | 41.06 | 72.17 | 106.78 | 156.63 | 78.28 | 181.17 |
| `epoll-spin` | interrupt | 2 | 589996 | 40.41 | 70.84 | 92.18 | 167.92 | 81.48 | 174.99 |
| `mio` | interrupt | 1 | 589994 | 40.66 | 86.46 | 107.85 | 216.59 | 97.34 | 218.48 |
| `mio` | interrupt | 2 | 589994 | 40.37 | 75.78 | 97.11 | 640.75 | 90.70 | 641.05 |
| `uring-spin` | interrupt | 1 | 589995 | 39.87 | 62.09 | 90.08 | 167.24 | 68.30 | 167.29 |
| `uring-spin` | interrupt | 2 | 589996 | 39.95 | 62.98 | 81.32 | 160.28 | 72.63 | 187.27 |

### irqlocal, N = 64 (1 order connection + 63 feed connections), 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589994 | 40.10 | 74.02 | 95.48 | 171.75 | 85.52 | 172.73 |
| `epoll-spin` | interrupt | 2 | 589994 | 39.95 | 78.55 | 101.00 | 155.48 | 91.03 | 168.38 |
| `mio` | interrupt | 1 | 589993 | 40.48 | 71.57 | 110.42 | 177.34 | 82.07 | 183.73 |
| `mio` | interrupt | 2 | 589993 | 41.24 | 72.42 | 109.32 | 172.45 | 82.40 | 174.53 |
| `uring-spin` | interrupt | 1 | 589994 | 40.31 | 80.57 | 105.59 | 183.18 | 91.94 | 197.02 |
| `uring-spin` | interrupt | 2 | 589994 | 40.49 | 68.26 | 108.33 | 185.46 | 82.70 | 187.77 |

## The HTTP order round trip through a pool

One request in flight, each on the pool's next connection in turn, every pooled connection on the loop; the server answers every connection from one thread. One clock (the receiver's). "From due" as above.

### irqcore, pool of 1 connection, 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589947 | 40.17 | 50.23 | 63.58 | 157.60 | 50.50 | 273.98 |
| `epoll-spin` | interrupt | 2 | 589946 | 40.37 | 51.15 | 58.53 | 136.44 | 51.42 | 136.70 |
| `mio` | interrupt | 1 | 590000 | 41.05 | 50.81 | 57.94 | 126.82 | 51.13 | 129.37 |
| `mio` | interrupt | 2 | 590000 | 41.06 | 51.36 | 58.70 | 225.10 | 51.68 | 225.45 |
| `uring-spin` | interrupt | 1 | 589949 | 40.25 | 50.13 | 56.94 | 137.96 | 50.39 | 138.20 |
| `uring-spin` | interrupt | 2 | 589945 | 41.03 | 51.17 | 58.43 | 150.16 | 51.44 | 150.43 |
| `recv-busy` | polling | 1 | 589948 | 38.63 | 47.62 | 53.38 | 132.87 | 47.89 | 145.62 |
| `recv-busy` | polling | 2 | 589946 | 38.51 | 48.16 | 54.60 | 128.74 | 48.44 | 129.01 |
| `recv-busy-pq` | polling | 1 | 589947 | 38.87 | 48.24 | 54.45 | 142.98 | 48.52 | 143.24 |
| `recv-busy-pq` | polling | 2 | 589947 | 39.03 | 49.03 | 56.10 | 147.98 | 49.30 | 148.25 |
| `uring-sqpoll` | polling | 1 | 589947 | 38.72 | 47.57 | 53.71 | 140.37 | 47.76 | 140.53 |
| `uring-sqpoll` | polling | 2 | 589946 | 38.97 | 48.59 | 55.25 | 123.21 | 48.77 | 123.39 |

### irqcore, pool of 20 connections, 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589948 | 42.33 | 52.81 | 59.57 | 136.74 | 53.08 | 137.02 |
| `epoll-spin` | interrupt | 2 | 589946 | 42.93 | 54.90 | 62.51 | 285.44 | 55.16 | 285.71 |
| `mio` | interrupt | 1 | 589999 | 42.32 | 52.96 | 59.66 | 156.66 | 53.28 | 156.99 |
| `mio` | interrupt | 2 | 589999 | 42.87 | 54.97 | 63.31 | 1478.01 | 55.32 | 1478.33 |
| `uring-spin` | interrupt | 1 | 589947 | 42.16 | 52.76 | 59.00 | 140.10 | 53.03 | 161.75 |
| `uring-spin` | interrupt | 2 | 589939 | 42.59 | 55.95 | 76.09 | 405.50 | 56.30 | 405.75 |
| `recv-busy` | polling | 1 | 589947 | 56.47 | 68.92 | 76.09 | 164.41 | 69.20 | 164.65 |
| `recv-busy` | polling | 2 | 589934 | 57.20 | 70.21 | 78.70 | 307.63 | 70.51 | 307.93 |
| `recv-busy-pq` | polling | 1 | 589947 | 48.89 | 60.73 | 66.74 | 149.66 | 61.01 | 149.96 |
| `recv-busy-pq` | polling | 2 | 589937 | 48.65 | 60.45 | 67.80 | 265.53 | 60.74 | 272.38 |
| `uring-sqpoll` | polling | 1 | 589947 | 42.85 | 53.52 | 62.12 | 248.27 | 53.79 | 348.73 |
| `uring-sqpoll` | polling | 2 | 589947 | 43.05 | 54.40 | 61.73 | 138.50 | 54.57 | 138.68 |

### irqlocal, pool of 1 connection, 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589946 | 40.02 | 49.95 | 57.60 | 113.17 | 50.24 | 155.83 |
| `epoll-spin` | interrupt | 2 | 589949 | 40.99 | 51.26 | 59.05 | 148.45 | 51.55 | 169.67 |
| `mio` | interrupt | 1 | 590000 | 40.38 | 50.16 | 57.18 | 180.45 | 50.52 | 180.83 |
| `mio` | interrupt | 2 | 590000 | 40.24 | 50.05 | 57.12 | 125.12 | 50.46 | 142.54 |
| `uring-spin` | interrupt | 1 | 589947 | 40.16 | 49.84 | 56.76 | 159.09 | 50.14 | 160.42 |
| `uring-spin` | interrupt | 2 | 589947 | 39.56 | 49.16 | 55.88 | 119.55 | 49.52 | 152.09 |

### irqlocal, pool of 20 connections, 10000/s

| backend | family | round | n | p50 | p99 | p99.9 | max | from due: p99 | from due: max |
|---|---|---|---|---|---|---|---|---|---|
| `epoll-spin` | interrupt | 1 | 589947 | 42.77 | 54.44 | 62.32 | 120.11 | 54.77 | 144.86 |
| `epoll-spin` | interrupt | 2 | 589948 | 42.52 | 55.02 | 70.14 | 347.21 | 55.36 | 363.02 |
| `mio` | interrupt | 1 | 589999 | 42.53 | 53.84 | 60.30 | 147.50 | 54.24 | 147.90 |
| `mio` | interrupt | 2 | 589999 | 42.45 | 54.76 | 70.06 | 270.09 | 55.17 | 316.34 |
| `uring-spin` | interrupt | 1 | 589947 | 42.17 | 54.26 | 68.31 | 410.31 | 54.59 | 410.59 |
| `uring-spin` | interrupt | 2 | 589948 | 42.00 | 53.59 | 60.24 | 156.03 | 53.89 | 156.37 |

