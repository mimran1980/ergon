# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|

## Diagnosis

Separate runs, slowed by their tracing (not timed comparisons): what the stalls the timed runs could not attribute were.

#### Kernel functions over 50 µs on the loop CPU: fgraph-pp-recv-busy

| function | count | max µs |
|---|---|---|
| `schedule` | 2 | 53250534.0 |
| `schedule_idle` | 2 | 52076301.0 |
| `do_idle` | 1 | 8207109.0 |
| `cpuidle_idle_call` | 2 | 57406.2 |
| `__x64_sys_write` | 1 | 40927.9 |
| `ksys_write` | 1 | 40927.7 |
| `vfs_write` | 1 | 40927.1 |
| `ext4_file_write_iter` | 1 | 40925.0 |
| `ext4_buffered_write_iter` | 1 | 40924.8 |
| `generic_perform_write` | 1 | 40919.6 |
| `__x64_sys_mmap` | 5 | 8958.6 |
| `ksys_mmap_pgoff` | 5 | 8958.4 |
| `vm_mmap_pgoff` | 5 | 8958.1 |
| `__mm_populate` | 6 | 8951.0 |
| `populate_vma_page_range` | 6 | 8950.2 |
| `__get_user_pages` | 6 | 8945.0 |
| `__x64_sys_recvmsg` | 540 | 2987.2 |
| `__sys_recvmsg` | 537 | 2987.0 |
| `___sys_recvmsg` | 526 | 2986.2 |
| `____sys_recvmsg` | 482 | 2985.7 |

#### Kernel functions over 50 µs on the loop CPU: fgraph-pp-uring-spin

| function | count | max µs |
|---|---|---|
| `schedule_idle` | 2 | 51920638.0 |
| `do_idle` | 1 | 8261673.0 |
| `cpuidle_idle_call` | 4 | 56509.6 |
| `__x64_sys_mmap` | 6 | 8173.8 |
| `ksys_mmap_pgoff` | 6 | 8173.6 |
| `vm_mmap_pgoff` | 6 | 8173.4 |
| `__mm_populate` | 6 | 8164.3 |
| `populate_vma_page_range` | 6 | 8163.5 |
| `__get_user_pages` | 6 | 8153.7 |
| `__x64_sys_write` | 1 | 3332.8 |
| `ksys_write` | 1 | 3332.6 |
| `vfs_write` | 1 | 3332.2 |
| `ext4_file_write_iter` | 1 | 3330.5 |
| `ext4_buffered_write_iter` | 1 | 3330.2 |
| `generic_perform_write` | 1 | 3325.5 |
| `__x64_sys_io_uring_enter` | 62 | 3282.7 |
| `io_cqring_wait` | 53 | 3282.2 |
| `__io_run_local_work` | 49 | 3281.6 |
| `__io_run_local_work_loop` | 45 | 3280.8 |
| `io_poll_task_func` | 44 | 3280.6 |

#### Kernel functions over 50 µs on the loop CPU: fgraph-pp-uring-sqpoll

| function | count | max µs |
|---|---|---|
| `schedule_idle` | 1 | 60184074.0 |
| `cpuidle_idle_call` | 3 | 50808.9 |
| `__x64_sys_mmap` | 5 | 8311.1 |
| `ksys_mmap_pgoff` | 5 | 8310.9 |
| `vm_mmap_pgoff` | 5 | 8310.7 |
| `__mm_populate` | 5 | 8304.3 |
| `populate_vma_page_range` | 5 | 8303.6 |
| `__get_user_pages` | 5 | 8299.0 |
| `__x64_sys_munmap` | 4 | 2077.5 |
| `__vm_munmap` | 4 | 2077.2 |
| `do_vmi_munmap` | 4 | 2076.5 |
| `do_vmi_align_munmap` | 4 | 2074.9 |
| `exit_mm` | 1 | 2071.0 |
| `mmput` | 1 | 2068.3 |
| `exit_mmap` | 1 | 2059.5 |
| `vms_complete_munmap_vmas` | 4 | 2027.0 |
| `unmap_vmas` | 3 | 1746.8 |
| `tlb_finish_mmu` | 4 | 1015.8 |
| `unmap_single_vma.isra.0` | 9 | 1004.4 |
| `unmap_page_range` | 9 | 1004.0 |

#### Hardware latency (hwlat) on CPUs 2,3,4,5,6,7, 120 s

Gaps with interrupts off (the hypervisor or firmware): 218 over 10 µs, 6 over 100 µs, 1 over 1 ms; longest 1054 µs.

| CPU | gaps | over 100 µs | over 1 ms | longest µs | gaps over 100 µs bunch at |
|---|---|---|---|---|---|
| 2 | 57 | 2 | 1 | 1054 | :52 (2 of 2, 2 minutes) |
| 3 | 49 | 2 | 0 | 667 | :52 (2 of 2, 2 minutes) |
| 4 | 45 | 2 | 0 | 778 | :52 (2 of 2, 2 minutes) |
| 5 | 29 | 0 | 0 | 62 |  |
| 6 | 23 | 0 | 0 | 64 |  |
| 7 | 15 | 0 | 0 | 61 |  |

#### Slow syscalls (over 100 µs) on the loop CPU: syscalls-pp-recv-busy

| syscall | count | max µs | total ms |
|---|---|---|---|
| mmap | 4 | 8329.7 | 12.66 |
| recvmsg | 9 | 716.4 | 3.31 |
| munmap | 2 | 208.1 | 0.40 |

Loop stalls over 100 µs: 10; inside a slow syscall: 9.

#### Slow syscalls (over 100 µs) on the loop CPU: syscalls-pp-uring-spin

| syscall | count | max µs | total ms |
|---|---|---|---|
| mmap | 4 | 8199.7 | 12.53 |
| io_uring_enter | 2 | 594.8 | 0.71 |
| munmap | 2 | 201.9 | 0.40 |
| syscall 425 | 1 | 336.3 | 0.34 |

Loop stalls over 100 µs: 8; inside a slow syscall: 2.

#### Slow syscalls (over 100 µs) on the loop CPU: syscalls-pp-uring-sqpoll

| syscall | count | max µs | total ms |
|---|---|---|---|
| mmap | 4 | 7963.2 | 12.26 |
| munmap | 2 | 208.3 | 0.37 |
| syscall 425 | 1 | 320.1 | 0.32 |

Loop stalls over 100 µs: 8; inside a slow syscall: 0.

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| cpumap-receiver/cpu2 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu3 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu4 | counter-only spinner | 5 | :52 | 5 (100%) | 2 of 2 | 762.65 | **yes** |
| cpumap-receiver/cpu5 | counter-only spinner | 3 | :52 | 3 (100%) | 2 of 2 | 879.34 | no |
| cpumap-receiver/cpu6 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu7 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu1 | counter-only spinner | 2 | :04 | 2 (100%) | 2 of 2 | 502.96 | no |
| diag/agents-off-1 | idle spare core | 0 | | | | | |
| diag/agents-off-1 | receiver loop | 5 | :52 | 4 (80%) | 3 of 3 | 673.42 | no |
| diag/agents-off-1 | sender loop | 4 | :04 | 4 (100%) | 3 of 3 | 441.11 | no |
| diag/agents-off-1 | slow samples | 7 | :04 | 4 (57%) | 3 of 3 | 580.90 | no |
| diag/agents-off-2 | idle spare core | 0 | | | | | |
| diag/agents-off-2 | receiver loop | 6 | :52 | 5 (83%) | 3 of 3 | 721.04 | **yes** |
| diag/agents-off-2 | sender loop | 5 | :04 | 5 (100%) | 3 of 3 | 437.68 | **yes** |
| diag/agents-off-2 | slow samples | 7 | :04 | 4 (57%) | 2 of 3 | 460.54 | no |
| diag/agents-on-1 | idle spare core | 0 | | | | | |
| diag/agents-on-1 | receiver loop | 8 | :52 | 7 (88%) | 3 of 3 | 872.20 | **yes** |
| diag/agents-on-1 | sender loop | 6 | :04 | 5 (83%) | 3 of 3 | 508.86 | **yes** |
| diag/agents-on-1 | slow samples | 15 | :51 | 7 (47%) | 3 of 3 | 972.86 | **yes** |
| diag/agents-on-2 | idle spare core | 0 | | | | | |
| diag/agents-on-2 | receiver loop | 3 | :52 | 2 (67%) | 2 of 3 | 674.50 | no |
| diag/agents-on-2 | sender loop | 5 | :04 | 5 (100%) | 3 of 3 | 539.69 | **yes** |
| diag/agents-on-2 | slow samples | 10 | :04 | 7 (70%) | 3 of 3 | 555.66 | **yes** |
| diag/fgraph-pp-recv-busy | receiver loop | 3 | :52 | 2 (67%) | 1 of 1 | 772.20 | no |
| diag/fgraph-pp-recv-busy | slow samples | 5 | :04 | 2 (40%) | 1 of 1 | 533.11 | no |
| diag/fgraph-pp-uring-spin | receiver loop | 3 | :52 | 2 (67%) | 1 of 1 | 1206.16 | no |
| diag/fgraph-pp-uring-spin | slow samples | 13 | :04 | 5 (38%) | 1 of 1 | 487.56 | **yes** |
| diag/fgraph-pp-uring-sqpoll | receiver loop | 2 | :52 | 2 (100%) | 1 of 1 | 998.50 | no |
| diag/fgraph-pp-uring-sqpoll | slow samples | 12 | :04 | 4 (33%) | 1 of 1 | 611.64 | no |
| diag/syscalls-pp-recv-busy | receiver loop | 4 | :52 | 3 (75%) | 2 of 2 | 672.89 | no |
| diag/syscalls-pp-recv-busy | sender loop | 6 | :04 | 6 (100%) | 2 of 2 | 555.97 | **yes** |
| diag/syscalls-pp-recv-busy | slow samples | 8 | :04 | 5 (62%) | 2 of 2 | 539.10 | **yes** |
| diag/syscalls-pp-uring-spin | receiver loop | 4 | :52 | 3 (75%) | 2 of 2 | 651.54 | no |
| diag/syscalls-pp-uring-spin | sender loop | 3 | :04 | 3 (100%) | 2 of 2 | 405.75 | no |
| diag/syscalls-pp-uring-spin | slow samples | 14 | :05 | 4 (29%) | 2 of 2 | 408.96 | no |
| diag/syscalls-pp-uring-sqpoll | receiver loop | 3 | :52 | 3 (100%) | 2 of 2 | 905.41 | no |
| diag/syscalls-pp-uring-sqpoll | sender loop | 3 | :04 | 3 (100%) | 2 of 2 | 653.50 | no |
| diag/syscalls-pp-uring-sqpoll | slow samples | 6 | :03 | 3 (50%) | 2 of 2 | 677.04 | no |

# Every run in detail

