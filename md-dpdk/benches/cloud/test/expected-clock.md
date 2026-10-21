# Clock benchmarks (clockbench)

## Read cost, ns: median of the batch means (the slowest batch's mean)

Each host in its most isolated state, the LTO build, node a: c7i.2xlarge: isolated, node a, x86_64, Intel(R) Xeon(R) Platinum 8488C; c8g.xlarge: isolated, node a, aarch64, implementer 0x41 part 0xd4f (Neoverse V2, Graviton4).

ergon-clock's ordered read (`ergon_ticks_ordered`, `ergon_now_ns_ordered`): c7i.2xlarge `rdtscp`; c8g.xlarge `isb; mrs cntvct_el0`.

| read | c7i.2xlarge | c8g.xlarge |
|---|---|---|
| `ergon_ticks` | 13.90 (16.00) | 12.10 (14.90) |
| `ergon_now_ns` | 14.80 (17.20) | 12.20 (15.10) |
| `raw_rdtsc` | 13.90 (15.90) | - |
| `quanta_now_ns` | 15.60 (18.00) | 13.40 (16.20) |
| `minstant_unix_ns` | FAILED its check | - |
| `raw_cntvct` | - | 12.10 (13.80) |

## Conversion alone, ns: median (slowest batch)

| conversion | c7i.2xlarge | c8g.xlarge |
|---|---|---|
| `convert_ergon_ticks_to_ns` | 0.90 (1.10) | 0.41 (0.60) |
| `convert_quanta_delta_as_nanos` | 1.40 (1.70) | 0.92 (1.20) |

## Built without LTO: ergon-clock's reads are #[inline], quanta's are calls

| read, ns | c8g.xlarge |
|---|---|
| `ergon_ticks` | - |
| `quanta_raw` | - |
| `ergon_now_ns` | 12.21 (15.00) |
| `quanta_now_ns` | 15.90 (18.30) |
| `quanta_now` | - |
| `ergon_now_ns_ordered` | - |
| `convert_ergon_ticks_to_ns` | 0.41 (0.60) |
| `convert_quanta_delta_as_nanos` | 2.60 (3.00) |

## Gaps between consecutive reads, ns: resolution, p50, p99.99, max (backwards steps)

| read | c7i.2xlarge | c8g.xlarge |
|---|---|---|
| `ergon_now_ns` | 0.4, 14.6, 30.1, 9800.0 (0) | 1.0, 12.0, 41.0, 18750.0 (0) |
| `ergon_ticks` | - | 1.0, 12.0, 40.0, 25040.0 (0) |
| `raw_cntvct` | - | 1.0, 12.0, 40.0, 12100.0 (0) |
| `quanta_now_ns` | - | 1.0, 13.0, 42.0, 11000.0 (0) |

## Across two threads on two CPUs: second reads lower than the first, of the handoffs (largest step back, ns; offset b over a, ns)

| read | c7i.2xlarge | c8g.xlarge |
|---|---|---|
| `ergon_ticks` | 0 of 200000 (0.0; -1.0) | 4 of 200000 (4.0; -1.0) |
| `ergon_now_ns` | - | 4 of 200000 (3.0; 0.0) |

## Drift against CLOCK_MONOTONIC_RAW: ppm at the last checkpoint (largest error, ns)

CLOCK_MONOTONIC_RAW: the kernel's unadjusted scaling of its clock source, which ergon-clock calibrates against; where that source is the counter, a counter clock's drift from it is how its conversion differs from the kernel's, not its accuracy.

| clock | c7i.2xlarge | c8g.xlarge |
|---|---|---|
| `ergon_now_ns` | - | -0.000 at 120 s (25) |
| `quanta_now_ns` | - | -4.211 at 120 s (505300) |

## Drift against CLOCK_MONOTONIC: ppm at the last checkpoint (largest error, ns)

CLOCK_MONOTONIC: steered by NTP (chrony) to its sources' rate, so independent of the counter's oscillator; a clock at the counter's raw rate drifts from it by the oscillator's error as NTP sees it, plus any slewing during the run.

| clock | c7i.2xlarge | c8g.xlarge |
|---|---|---|
| `ergon_now_ns` | - | -2.500 at 120 s (300000) |
| `quanta_now_ns` | - | -6.711 at 120 s (805300) |

## Calibration in fresh processes: ms min / median / max; the rate's spread, ppm

| calibration | c7i.2xlarge | c8g.xlarge |
|---|---|---|
| ergon | 70.300 / 70.500 / 72.100 ms; 0.000 ppm (hypervisor) | 40.100 / 40.200 / 41.900 ms; 0.000 ppm (register) |
| quanta | - | 6.100 / 7.800 / 200.010 ms; 1.700 ppm (-) |

## Gates and comparisons, ns

|  | c7i.2xlarge | c8g.xlarge |
|---|---|---|
| gate `unordered_ns_over_bare_read` | 0.90 (limit 1.00): pass | 0.10 (limit 1.00): pass |
| gate `ordered_ticks_over_bare_ordered_read` | 1.30 (limit 1.00): fail, against `raw_rdtscp` | 0.20 (limit 1.00): pass, against `raw_isb_cntvct` |
| `ergon_now_ns_minus_quanta_now_ns` | -0.80 | -1.20 |

## Every run

| host | node | state | build | arch | clocksource | ergon-clock (cross-check) | default (kernel) | ECV | ordered read | failed checks |
|---|---|---|---|---|---|---|---|---|---|---|
| c8g.xlarge | a | isolated | lto | aarch64 | arch_sys_counter | counter register 1000000000 Hz | counter (none) | no | isb; mrs cntvct_el0 | none |
| c8g.xlarge | a | isolated | nolto | aarch64 | - | counter register 1000000000 Hz | counter (none) | - | - | none |
| c8g.xlarge | a | pinned | lto | aarch64 | - | counter register 1000000000 Hz | counter (none) | - | - | none |
| c7i.2xlarge | a | isolated | lto | x86_64 | tsc | counter hypervisor 2400000000 Hz | counter (none) | - | rdtscp | minstant_unix_ns |
| c7i.2xlarge | b | isolated | lto | x86_64 | - | counter hypervisor 2400000000 Hz | counter (none) | - | - | none |

## Each read's longest gap (10 us or more), attributed

A traced preemption or interrupt is matched within 10 ms of a gap (the trace's wall-clock anchor, /proc/uptime, has that resolution); the gap spinner's stalls exactly.

| host | node | state | build | read | gap, us | cause |
|---|---|---|---|---|---|---|
| c8g.xlarge | a | isolated | lto | `ergon_ticks` | 25.0 | unexplained |
| c8g.xlarge | a | isolated | lto | `ergon_now_ns` | 18.8 | preempted x1 (by kworker/1:1) |
| c8g.xlarge | a | isolated | lto | `raw_cntvct` | 12.1 | host stall (the gap spinner stalled across it) |
| c8g.xlarge | a | isolated | lto | `quanta_now_ns` | 11.0 | interrupt x1 (irq=12 name=ens6-Tx-Rx-0) |

## ergon-clock's discipline to CLOCK_REALTIME (clockbench --discipline): the residual, ns

The absolute residual, CLOCK_REALTIME less the disciplined clock, p50 / p99 / max: at the owner's samples, one a second (what each update found before it re-anchored the clock; held updates left out), and from the verifier, a follower's copy of the clock bracketing CLOCK_REALTIME between two of its ordered reads 100 times a second (a bracket over ten times the median width was interrupted: counted apart, with the widest). The verifier's max is placed after the last update before it, with that update's action and flags. `start`: what the first update did (a clock behind the wall clock is stepped forward then, by design, so a start step is no anomaly); `rejected`: samples whose bracket was too wide, the clock held at its frequency estimate; `steps after start`: the clock stepped forward after the first update; `wall steps`: CLOCK_REALTIME stepped under it (chrony).

| host | node | state | s | start | at samples | verifier | verifier max at | interrupted | rejected | steps after start | wall steps | update cost |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| c8g.xlarge | a | isolated | 5 | slewed | 420 / 850 / 2600 | 300 / 700 / 2500 | 3.20 s, after the update at 3.00 s (stepped:2000; wall-step, stepped) | 1 (widest 9000) | 1 | 1 | 1 | 4000 / 4300 / 5200 |
| c8g.xlarge | b | isolated | 2 | stepped 150 ns | 10 / 10 / 150 | 8 / 8 / 25 | 0.50 s, after the update at 0.00 s (stepped:150; stepped, start) | 0 | 0 | 0 | 0 | 3050 / 3050 / 3100 |

## chrony's tracking around each discipline run (vm.sh discipline: chronyc tracking before and after)

| host | node | state | before | after |
|---|---|---|---|---|
| c8g.xlarge | a | isolated | 169.254.169.123 (stratum 4): system time +123 ns, last offset +45 ns, RMS offset 310 ns, frequency 12.345 ppm slow, skew 0.012 ppm, leap Normal | 169.254.169.123 (stratum 4): system time -80 ns, last offset -20 ns, RMS offset 290 ns, frequency 12.346 ppm slow, skew 0.010 ppm, leap Normal |
| c8g.xlarge | b | isolated | - | - |

