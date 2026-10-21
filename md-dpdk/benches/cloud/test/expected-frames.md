## Throughput phase frames

From vm.sh counters on both hosts just before and after each flat-out run. Frames are host a's sent (the transport's `pkts` over both ports; the data NICs' `tx_packets` in kernel modes) and host b's received, as rates over the subscriber's warm-up second and samples (`tput_seconds` + 1 s), the only time the publisher has a receiver window. Messages per frame divides the run's msgs/s (over several flows, their aggregate) by the frames sent, so frames sent twice lower it. `tx-queued` (frames a full ring refused and the transport queued to go first) and `tx-eagain` (frames refused back to Aeron) are host a's growth over the run.

| state, transport | run | rep | msgs/s (M) | frames/s sent by a (M) | msgs per frame | bytes per frame | frames/s received by b (M) | sent − received | tx-queued | tx-eagain |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| isolated, kernel UDP, busy polling | 32 B flat out | 1 | 13.50 | 0.930 | 14.5 | - | 0.930 | 0 | - | - |
| isolated, DPDK | 32 B flat out | 1 | 10.00 | 0.700 | 14.3 | 1,400 | 0.700 | 1,000 | 3,000 | 7 |
| isolated, DPDK | 288 B sustained maximum | 1 | 2.40 | 0.333 | 7.2 | 1,408 | 0.333 | 0 | 0 | 0 |
| isolated, DPDK | 288 B, 2 flows at once | 1 | 4.70 | 0.700 | 6.7 | 1,380 | 0.700 | 1,000 | 0 | 5 |
| isolated, DPDK | 288 B, 2 flows at once, left out (overlap 0.899) | 2 | - | 0.800 | - | 1,380 | 0.798 | 10,000 | 0 | 0 |
| isolated, dpdk@BIN=bin-base | 32 B flat out | 1 | 6.50 | 1.083 | 6.0 | 1,000 | 1.067 | 100,000 | - | 250,000 |
