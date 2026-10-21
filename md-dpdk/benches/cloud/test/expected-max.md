## Sustained one-way maximum by message size (M msgs/s)

`flat-out` is the most the publisher sustained with nothing held back: a ceiling, lossy when its loss counters grew. `loss-free` is the highest rate of the ladder below the first point that lost anything in any rep: a loss counter grew on either host, short sends included (those an ARP miss explains aside); a message went unechoed; or a loss read failed or was not taken. The transport's own send-side counters (a full TX ring, an ARP miss, no mbuf, a full kernel send buffer) do not count on their own: each send they refuse is also a short send, and a full ring that queues a vector refuses nothing. Nor does `isolated-nic-irqs`, the data NICs' interrupts on CPUs other than the housekeeping one: a check on the run (under the loss counters' tables), listed with the loss counters but not loss. `keeps up` is the highest rate below the first point where a rep achieved under 98% of the offered rate (saturation, not loss), and `sustainable` the lower of the two.

| state, transport | size | reps | flat-out msgs/s | flat-out loss counters | loss-free | keeps up | sustainable |
| --- | --- | --- | --- | --- | --- | --- | --- |
| isolated, DPDK | 288 B | 3 | 2.45 (2.40–2.50) | none | 250,000/s | 250,000/s | 250,000/s |
| isolated, localhost, DPDK driver | 288 B | 1 | 3.00 (3.00–3.00) | not sampled | no ladder | no ladder | no ladder |
| isolated, dpdk@BIN=bin-base | 288 B | 3 | 1.90 (1.80–2.42) | naks-sent 1,500 | none (250,000/s lost) | 250,000/s | none |
