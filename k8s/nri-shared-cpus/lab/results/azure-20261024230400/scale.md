# Shared CPUs: how much a pod gets

## a-burst-100m: one pod: cpu 1 exclusive, 100 shared millicpus, 4 busy helper threads

Expected: its helpers on 1 of the 2 shared CPUs (ceil(100/1000) = 1), using up to all of it: about 1 core, nr_throttled 0, the other shared CPU idle

Window 30 s. Busy CPU from /proc/stat on the VM (every task, not only the pods').

| CPU | role | busy, cores |
|---|---|---|
| 1 | shared | 1.00 |
| 2 | shared | 0.05 |
| 3 | exclusive | 1.00 |
| 4 | exclusive | 0.00 |
| 5 | exclusive | 0.00 |
| 6 | exclusive | 0.00 |
| 7 | exclusive | 0.00 |

Shared CPUs busy: 1.05 of 2 cores.

| pod | exclusive cpu | shared-millicpus | its shared CPUs | helper threads x duty % | cpu.weight container / pod | cpu.max container / pod | helpers' CPU, cores | share of the shared CPUs' busy time | nr_periods / nr_throttled / throttled_usec grown, container | the same, pod | spin max µs | spin involuntary switches | spin thread's CPUs (samples) | its other threads' CPUs (samples) | placement |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| scale-100m | 1 | 100 | 1 | 4 x 100 | 100 / 39 | max 100000 / max 100000 | 0.99 | 94 % | 0 / 0 / 0 | 0 / 0 / 0 | 15.08 | 0 | 3 (28) | 1 (196) | ok |

## b-burst-1001m: one pod: cpu 1 exclusive, 1001 shared millicpus, 4 busy helper threads

Expected: its helpers on both shared CPUs (ceil(1001/1000) = 2): about 2 cores, nr_throttled 0

Window 30 s. Busy CPU from /proc/stat on the VM (every task, not only the pods').

| CPU | role | busy, cores |
|---|---|---|
| 1 | shared | 1.00 |
| 2 | shared | 1.00 |
| 3 | exclusive | 1.00 |
| 4 | exclusive | 0.00 |
| 5 | exclusive | 0.00 |
| 6 | exclusive | 0.00 |
| 7 | exclusive | 0.00 |

Shared CPUs busy: 2.00 of 2 cores.

| pod | exclusive cpu | shared-millicpus | its shared CPUs | helper threads x duty % | cpu.weight container / pod | cpu.max container / pod | helpers' CPU, cores | share of the shared CPUs' busy time | nr_periods / nr_throttled / throttled_usec grown, container | the same, pod | spin max µs | spin involuntary switches | spin thread's CPUs (samples) | its other threads' CPUs (samples) | placement |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| scale-1001m | 1 | 1001 | 1,2 | 4 x 100 | 100 / 39 | max 100000 / max 100000 | 1.99 | 99 % | 0 / 0 / 0 | 0 / 0 / 0 | 16.53 | 0 | 3 (28) | 1 (112), 2 (84) | ok |

## c-idle: one pod: cpu 1 exclusive, 100 shared millicpus, 4 helper threads busy 1 % of each 10 ms

Expected: the pod's helpers near 0.04 cores

Window 30 s. Busy CPU from /proc/stat on the VM (every task, not only the pods').

| CPU | role | busy, cores |
|---|---|---|
| 1 | shared | 0.05 |
| 2 | shared | 0.02 |
| 3 | exclusive | 1.00 |
| 4 | exclusive | 0.00 |
| 5 | exclusive | 0.00 |
| 6 | exclusive | 0.00 |
| 7 | exclusive | 0.00 |

Shared CPUs busy: 0.08 of 2 cores.

| pod | exclusive cpu | shared-millicpus | its shared CPUs | helper threads x duty % | cpu.weight container / pod | cpu.max container / pod | helpers' CPU, cores | share of the shared CPUs' busy time | nr_periods / nr_throttled / throttled_usec grown, container | the same, pod | spin max µs | spin involuntary switches | spin thread's CPUs (samples) | its other threads' CPUs (samples) | placement |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| scale-idle | 1 | 100 | 1 | 4 x 1 | 100 / 39 | max 100000 / max 100000 | 0.04 | 51 % | 0 / 0 / 0 | 0 / 0 / 0 | 16.44 | 0 | 3 (28) | 1 (196) | ok |

## d-contention: three pods, 100 shared millicpus and 4 busy helper threads each, started in turn: A (cpu 1) and B (cpu 1) get the two shared CPUs, C (cpu 2) the least loaded, A's

Expected: A and C share one shared CPU, split by their cgroups' cpu.weight (from the cpu request: C's twice A's), not by their equal millicpus; B alone on the other

Window 31 s. Busy CPU from /proc/stat on the VM (every task, not only the pods').

| CPU | role | busy, cores |
|---|---|---|
| 1 | shared | 1.00 |
| 2 | shared | 1.00 |
| 3 | exclusive | 1.00 |
| 4 | exclusive | 1.00 |
| 5 | exclusive | 1.00 |
| 6 | exclusive | 0.00 |
| 7 | exclusive | 0.00 |

Shared CPUs busy: 2.00 of 2 cores.

| pod | exclusive cpu | shared-millicpus | its shared CPUs | helper threads x duty % | cpu.weight container / pod | cpu.max container / pod | helpers' CPU, cores | share of the shared CPUs' busy time | nr_periods / nr_throttled / throttled_usec grown, container | the same, pod | spin max µs | spin involuntary switches | spin thread's CPUs (samples) | its other threads' CPUs (samples) | placement |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| scale-a | 1 | 100 | 1 | 4 x 100 | 100 / 39 | max 100000 / max 100000 | 0.33 | 16 % | 0 / 0 / 0 | 0 / 0 / 0 | 14.96 | 0 | 3 (25) | 1 (175) | ok |
| scale-b | 1 | 100 | 2 | 4 x 100 | 100 / 39 | max 100000 / max 100000 | 0.99 | 49 % | 0 / 0 / 0 | 0 / 0 / 0 | 14.16 | 0 | 4 (25) | 2 (175) | ok |
| scale-c | 2 | 100 | 1 | 4 x 100 | 174 / 79 | max 100000 / max 100000 | 0.66 | 33 % | 0 / 0 / 0 | 0 / 0 / 0 | 11.73 | 0 | 5 (25) | 1 (175) | ok |

## accounting: the scheduler counts ergon.io/shared-millicpus

- ok:   allocatable ergon.io/shared-millicpus 2000 (sharedCpus 1-2, oversubscription 1)
- ok:   a pod asking 2100 stays Pending: 0/1 nodes are available: 1 Insufficient ergon.io/shared-millicpus. no new claims to deallocate, preemption: 0/1 nodes are available: 1 Preemption is not helpful for scheduling.
- ok:   a pod asking 2000 schedules (node ergon-lab-control-plane)
