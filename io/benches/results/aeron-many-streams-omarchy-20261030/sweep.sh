#!/bin/bash
# sweep.sh: the layout, sharding, concurrent, poller-threshold and term-length runs, in order,
# each under the bench lock (bench-run.sh), with syscalls counted per thread by sysccount.so.
R=$HOME/ergon-work/rusteron-readiness
run() { name=$1; shift; mkdir -p "$R/$name"; "$R/bench-run.sh" "$name" LD_PRELOAD="$R/sysccount.so" SYSCOUNT_DIR="$R/$name-sys" "$@" > /dev/null 2>&1; echo "$name: $(tail -1 "$R/$name/stdout.txt")"; }
for d in L B1 B2 C D E64K E16M; do rm -rf "$R/$d-sys"; mkdir -p "$R/$d-sys"; done
run L MANY_STREAMS_ARMS=idle,idle-sameport,idle-samestream,plain,sameport,samestream,mux,mux4,mux8 MANY_STREAMS_IDLE_N=1,32,64,128 MANY_STREAMS_TRAFFIC_N=1,32,64,128
run B1 MANY_STREAMS_ARMS=shard1,snshard1 MANY_STREAMS_TRAFFIC_N=1,32,64,128
run B2 MANY_STREAMS_ARMS=shard2,snshard2,snshard4 MANY_STREAMS_TRAFFIC_N=32,64,128
run C MANY_STREAMS_CONCURRENT=1 MANY_STREAMS_ARMS=sameport,mux,mux8 MANY_STREAMS_TRAFFIC_N=32,64,128
run D AERON_UDP_CHANNEL_TRANSPORT_POLLER_ITERATION_THRESHOLD=0 MANY_STREAMS_ARMS=idle,idle-sameport,sameport,mux8 MANY_STREAMS_IDLE_N=1,4,32 MANY_STREAMS_TRAFFIC_N=32,128
run E64K MANY_STREAMS_TERM_LENGTH=65536 MANY_STREAMS_ARMS=plain MANY_STREAMS_TRAFFIC_N=128
run E16M MANY_STREAMS_TERM_LENGTH=16777216 MANY_STREAMS_ARMS=plain MANY_STREAMS_TRAFFIC_N=128
echo SWEEP-DONE
