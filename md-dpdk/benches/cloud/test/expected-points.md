## Loss by load point (reps that lost anything, of the reps run; NAKs sent)

A rep lost something when a loss counter grew on either host (short sends included, those an ARP miss explains aside; `sat-`, `noport-`, the run-validity counters and the send-side causes on their own aside), a message went unechoed, or a loss read failed or was not taken, as `loss-free` counts it. In brackets, the NAKs both hosts sent over those reps. Compare variants on these counts: one rare event moves the strict `loss-free` rate by whole ladder steps.

| state, transport | size | 250,000/s | bursts |
| --- | --- | --- | --- |
| isolated, DPDK | 288 B | 0/3 | 1/3 (12) |
| isolated, dpdk@BIN=bin-base | 288 B | 2/3 (7) | 1/3 (7) |
