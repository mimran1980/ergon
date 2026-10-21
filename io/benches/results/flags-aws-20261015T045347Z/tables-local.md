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
| `schedule_idle` | 1 | 60662281.0 |
| `__x64_sys_mmap` | 5 | 102417.5 |
| `ksys_mmap_pgoff` | 5 | 102417.3 |
| `vm_mmap_pgoff` | 5 | 102417.1 |
| `__mm_populate` | 6 | 102389.9 |
| `populate_vma_page_range` | 6 | 102388.4 |
| `__get_user_pages` | 6 | 102377.2 |
| `__x64_sys_write` | 1 | 50985.7 |
| `ksys_write` | 1 | 50985.5 |
| `vfs_write` | 1 | 50984.9 |
| `ext4_file_write_iter` | 1 | 50982.9 |
| `ext4_buffered_write_iter` | 1 | 50982.6 |
| `generic_perform_write` | 1 | 50977.3 |
| `__x64_sys_munmap` | 5 | 44799.8 |
| `__vm_munmap` | 5 | 44799.5 |
| `do_vmi_munmap` | 5 | 44798.8 |
| `do_vmi_align_munmap` | 5 | 44797.5 |
| `vms_complete_munmap_vmas` | 5 | 44780.4 |
| `unmap_region` | 5 | 44778.3 |
| `unmap_vmas` | 6 | 42026.8 |

#### Kernel functions over 50 µs on the loop CPU: fgraph-pp-uring-spin

| function | count | max µs |
|---|---|---|
| `schedule_idle` | 1 | 60420681.0 |
| `__x64_sys_mmap` | 6 | 77636.0 |
| `ksys_mmap_pgoff` | 6 | 77635.8 |
| `vm_mmap_pgoff` | 6 | 77635.6 |
| `__mm_populate` | 7 | 77626.2 |
| `populate_vma_page_range` | 7 | 77625.5 |
| `__get_user_pages` | 6 | 77602.4 |
| `__x64_sys_munmap` | 5 | 34305.5 |
| `__vm_munmap` | 5 | 34305.3 |
| `do_vmi_munmap` | 5 | 34304.8 |
| `do_vmi_align_munmap` | 5 | 34304.6 |
| `vms_complete_munmap_vmas` | 5 | 34298.3 |
| `unmap_region` | 5 | 34296.9 |
| `unmap_vmas` | 6 | 24465.1 |
| `unmap_single_vma` | 14 | 24464.8 |
| `unmap_page_range` | 14 | 24464.5 |
| `tlb_finish_mmu` | 6 | 9824.0 |
| `cpuidle_idle_call` | 20 | 4064.1 |
| `call_cpuidle` | 20 | 4055.7 |
| `cpuidle_enter` | 20 | 4054.8 |

#### Kernel functions over 50 µs on the loop CPU: fgraph-pp-uring-sqpoll

| function | count | max µs |
|---|---|---|
| `schedule_idle` | 1 | 60403681.0 |
| `__x64_sys_mmap` | 5 | 77807.0 |
| `ksys_mmap_pgoff` | 5 | 77806.8 |
| `vm_mmap_pgoff` | 5 | 77806.6 |
| `__mm_populate` | 5 | 77797.8 |
| `populate_vma_page_range` | 5 | 77797.1 |
| `__get_user_pages` | 5 | 77773.9 |
| `__x64_sys_munmap` | 4 | 34442.3 |
| `__vm_munmap` | 4 | 34442.1 |
| `do_vmi_munmap` | 4 | 34441.7 |
| `do_vmi_align_munmap` | 4 | 34441.4 |
| `vms_complete_munmap_vmas` | 4 | 34435.2 |
| `unmap_region` | 4 | 34433.8 |
| `unmap_vmas` | 5 | 24557.1 |
| `unmap_single_vma` | 12 | 24556.7 |
| `unmap_page_range` | 12 | 24556.5 |
| `tlb_finish_mmu` | 5 | 9869.4 |
| `cpuidle_idle_call` | 21 | 4053.6 |
| `call_cpuidle` | 21 | 4045.0 |
| `cpuidle_enter` | 21 | 4044.0 |

#### Hardware latency (hwlat) on CPUs 1,5,2,6,3,7, 120 s

Gaps with interrupts off (the hypervisor or firmware): 140 over 10 µs, 0 over 100 µs, 0 over 1 ms; longest 57 µs.

| CPU | gaps | over 100 µs | over 1 ms | longest µs | gaps over 100 µs bunch at |
|---|---|---|---|---|---|
| 1 | 28 | 0 | 0 | 32 |  |
| 2 | 32 | 0 | 0 | 57 |  |
| 3 | 30 | 0 | 0 | 29 |  |
| 5 | 20 | 0 | 0 | 51 |  |
| 6 | 16 | 0 | 0 | 54 |  |
| 7 | 14 | 0 | 0 | 14 |  |

#### Slow syscalls (over 100 µs) on the loop CPU: syscalls-pp-recv-busy

| syscall | count | max µs | total ms |
|---|---|---|---|
| mmap | 4 | 14129.2 | 21.30 |
| munmap | 3 | 3488.3 | 5.25 |

Loop stalls over 100 µs: 0; inside a slow syscall: 0.

#### Slow syscalls (over 100 µs) on the loop CPU: syscalls-pp-uring-spin

| syscall | count | max µs | total ms |
|---|---|---|---|
| mmap | 4 | 14056.5 | 21.16 |
| munmap | 3 | 3580.7 | 5.43 |
| syscall 425 | 1 | 470.0 | 0.47 |

Loop stalls over 100 µs: 0; inside a slow syscall: 0.

#### Slow syscalls (over 100 µs) on the loop CPU: syscalls-pp-uring-sqpoll

| syscall | count | max µs | total ms |
|---|---|---|---|
| mmap | 4 | 14076.9 | 21.14 |
| munmap | 3 | 3615.6 | 5.48 |
| syscall 425 | 1 | 527.0 | 0.53 |

Loop stalls over 100 µs: 1; inside a slow syscall: 0.

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| cpumap-receiver/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu2 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu3 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu5 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu6 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu7 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu3 | counter-only spinner | 0 | | | | | |
| diag/agents-off-1 | idle spare core | 1 | :16 | 1 (100%) | 1 of 3 | 329.54 | no |
| diag/agents-off-1 | receiver loop | 0 | | | | | |
| diag/agents-off-1 | sender loop | 0 | | | | | |
| diag/agents-off-1 | slow samples | 1 | :00 | 1 (100%) | 1 of 3 | 572.84 | no |
| diag/agents-off-2 | idle spare core | 0 | | | | | |
| diag/agents-off-2 | receiver loop | 0 | | | | | |
| diag/agents-off-2 | sender loop | 0 | | | | | |
| diag/agents-off-2 | slow samples | 0 | | | | | |
| diag/agents-on-1 | idle spare core | 0 | | | | | |
| diag/agents-on-1 | receiver loop | 0 | | | | | |
| diag/agents-on-1 | sender loop | 0 | | | | | |
| diag/agents-on-1 | slow samples | 1 | :50 | 1 (100%) | 1 of 3 | 1048.32 | no |
| diag/agents-on-2 | idle spare core | 0 | | | | | |
| diag/agents-on-2 | receiver loop | 0 | | | | | |
| diag/agents-on-2 | sender loop | 0 | | | | | |
| diag/agents-on-2 | slow samples | 1 | :03 | 1 (100%) | 1 of 3 | 969.42 | no |
| diag/fgraph-pp-recv-busy | receiver loop | 0 | | | | | |
| diag/fgraph-pp-recv-busy | slow samples | 1 | :30 | 1 (100%) | 1 of 1 | 321.10 | no |
| diag/fgraph-pp-uring-spin | receiver loop | 0 | | | | | |
| diag/fgraph-pp-uring-spin | slow samples | 6 | :10 | 2 (33%) | 1 of 1 | 527.26 | no |
| diag/fgraph-pp-uring-sqpoll | receiver loop | 0 | | | | | |
| diag/fgraph-pp-uring-sqpoll | slow samples | 6 | :08 | 2 (33%) | 1 of 1 | 442.24 | no |
| diag/syscalls-pp-recv-busy | receiver loop | 0 | | | | | |
| diag/syscalls-pp-recv-busy | sender loop | 0 | | | | | |
| diag/syscalls-pp-recv-busy | slow samples | 0 | | | | | |
| diag/syscalls-pp-uring-spin | receiver loop | 0 | | | | | |
| diag/syscalls-pp-uring-spin | sender loop | 0 | | | | | |
| diag/syscalls-pp-uring-spin | slow samples | 4 | :01 | 2 (50%) | 2 of 2 | 512.26 | no |
| diag/syscalls-pp-uring-sqpoll | receiver loop | 0 | | | | | |
| diag/syscalls-pp-uring-sqpoll | sender loop | 0 | | | | | |
| diag/syscalls-pp-uring-sqpoll | slow samples | 1 | :39 | 1 (100%) | 1 of 2 | 956.50 | no |

# Every run in detail

