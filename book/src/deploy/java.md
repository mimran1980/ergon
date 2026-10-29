# Java for Aeron Archive and Cluster

`ergo-aeron-cluster` is only a client. The cluster (consensus module,
clustered services, Archive) runs on Aeron's Java, as does an Archive beside
a C media driver. This page: which Java, and the JVM and Aeron settings for
latency, for Aeron 1.54.0 (the release ergon pins). Each setting comes from
Aeron's own scripts, benchmarks or wiki ([sources](#sources)); none has a
latency A/B here.

Terms: [deployment glossary](overview.md#glossary),
[network and DPDK glossary](../md-dpdk/overview.md#glossary).

## Which Java

| Use | Java | Why |
|---|---|---|
| Run Archive and Cluster | **25**: Eclipse Temurin 25 (LTS) | The newest LTS in Aeron 1.54.0's CI (17, 21, 25); ergon's cluster harness runs on it in CI. `eclipse-temurin:25-jre` covers amd64 and arm64 (Graviton); Debian 13 ships `openjdk-25-jre-headless`. |
| Oldest that works | 17 | Aeron compiles its classes with `--release 17`. |
| Newest GA release, 27 | Runs, but not recommended | 27 is not an LTS release and is not in Aeron's test matrix. |
| Build Aeron's jars from source | **JDK 25** (17 to 27 work) | CI builds them on 25. Aeron's Gradle (9.8.2) runs on 17 to 27. Any JDK makes Java 17 bytecode. Or skip Gradle: `aeron-all-1.54.0.jar` on Maven Central holds Archive, Cluster and the samples. |

The `eclipse-temurin:25-jre` image has neither `wget` nor `curl`. Fetch the
jar with a checksummed `ADD`:

```dockerfile
FROM eclipse-temurin:25-jre
ADD --checksum=sha256:eed9fad510778af55c69cbe5b43ddc7ba754f79f8d592268c63fc3ad9d25707b --chmod=644 \
    https://repo1.maven.org/maven2/io/aeron/aeron-all/1.54.0/aeron-all-1.54.0.jar /aeron-all.jar
```

## Ready-made settings

Two settings files, both tested by
`cluster-test-harness/tests/jvm_settings.rs`: the JDK takes every option
without a warning, every property is one Aeron's sources name (Aeron ignores
a misspelt one), and a cluster node started with both serves a client.

```sh
java @aeron.jvm -cp aeron-all-1.54.0.jar \
    io.aeron.cluster.ClusteredMediaDriver low-latency.properties cluster.properties
```

`@aeron.jvm` is a java argument file. Archive, ArchivingMediaDriver,
ClusteredMediaDriver and ClusteredServiceContainer load every properties file
named on their command line, so `low-latency.properties` and your cluster
settings go there (or pass each as `-D<name>=<value>`).

Adapt two settings in `cluster-test-harness/jvm/aeron.jvm`: the heap
(`-Xms2g -Xmx2g`) is a placeholder, so size it for your clustered service's
state (Archive alone needs far less); the log goes to `/dev/shm`, which only
Linux has.

```text
{{#include ../../../cluster-test-harness/jvm/aeron.jvm}}
```

`cluster-test-harness/jvm/low-latency.properties`:

```properties
{{#include ../../../cluster-test-harness/jvm/low-latency.properties}}
```

### The JVM options

| Option | Why | Source |
|---|---|---|
| `--add-opens java.base/jdk.internal.misc=ALL-UNNAMED` | Required. Agrona 2.x reaches `jdk.internal.misc.Unsafe`; without this option the JVM fails with `IllegalAccessError`. | Aeron's scripts and `build.gradle` |
| `--add-opens java.base/java.util.zip=ALL-UNNAMED` | Required once Archive checksums are on (`aeron.archive.record.checksum`); without it they throw `InaccessibleObjectException`. | Aeron's scripts and `build.gradle` |
| `-XX:+UseParallelGC` | Archive, Cluster and the driver allocate nothing in steady state. The choice is convention, not a measured collector A/B: every Aeron script uses it. | every Aeron script |
| `-Xms` = `-Xmx`, `-XX:+AlwaysPreTouch` | The heap never grows, and every page is faulted in at start rather than on the hot path. | benchmarks |
| `-XX:MaxMetaspaceSize=1g`, `-XX:ReservedCodeCacheSize=1g` | Code-cache pressure never deoptimises hot code. | benchmarks |
| `-Xlog:async`, `-Xlog:gc*,safepoint=info,...` | Every pause and safepoint is logged with its cause, by a background thread, to tmpfs. | benchmarks (to `/dev/shm`); `-Xlog:async` is ergon's addition |
| `-Djava.net.preferIPv4Stack=true` | The Java driver uses IPv4 UDP sockets. | benchmarks, wiki, `build.gradle` |
| `-Dagrona.disable.bounds.checks=true` | Turns off the bounds checks in Agrona's buffers. It is read once, when the class loads, so it belongs on the command line. | `low-latency.properties`, benchmarks |

### The Aeron properties

| Property | Default | Why |
|---|---|---|
| `aeron.archive.idle.strategy=spin`, `aeron.archive.recorder.idle.strategy=noop`, `aeron.archive.replayer.idle.strategy=noop` | backoff | The Archive's conductor, recorder and replayer never sleep. Each one needs an isolated core. |
| `aeron.cluster.idle.strategy=noop` | backoff | The consensus module and the clustered service both read this property. |
| `aeron.term.buffer.sparse.file=false`, `aeron.pre.touch.mapped.memory=true` | sparse, no pre-touch | The first write to a page of a log buffer does not take a page fault. Aeron's own sources disagree here: its `low-latency.properties` sets both, while its benchmarks keep the defaults. Pre-touching charges every mapped term to the process's memory, and so to a container's limit. With the C media driver, the driver's own setting is `AERON_TERM_BUFFER_SPARSE_FILE`. |
| `aeron.thread.naming=new` | classic | Gives thread names short enough for Linux's 15 characters, so each hot thread can be pinned by name (see below). |
| `aeron.archive.*.cycle.threshold`, `aeron.cluster.cycle.threshold`, `aeron.cluster.service.cycle.threshold` = `1ms` | 100 ms | Counts every duty cycle longer than 1 ms, so stalls are visible. |

These settings are left at their defaults:

- `aeron.archive.file.sync.level` and `aeron.archive.catalog.file.sync.level`
  stay at `0`: writes go to the OS page cache only. `1` adds fdatasync and `2`
  adds fsync. Keep the catalog's level at least the archive's. Level `0`
  survives a process dying and loses whatever is still in the page cache if
  the machine does. In a 3- or 5-member cluster another member's log covers
  that loss, so `0` is the usual choice. A single-member cluster's quorum is
  one: its local log is the only copy, so raise both sync levels together
  there and pay the fsync.
- `aeron.archive.file.io.max.length` stays at `1m`.
- `aeron.archive.control.term.buffer.sparse` stays at `true`.
- `aeron.spies.simulate.connection` stays `false`. It is a media driver
  setting, which Aeron's benchmarks set to `true` on their driver. It matters
  when an Archive records a network publication by spying and nothing else
  subscribes: with `false` the publication waits for a network subscriber
  before it is connected.

## Pinning the hot threads

Java has no CPU-affinity setting. Aeron's benchmarks start the JVM on the
housekeeping cores with `numactl --membind --cpunodebind --physcpubind`. They
then move each hot thread to an isolated core by name with `taskset -p`.
With `aeron.thread.naming=new` the names are `aeron-ar-cnd`, `aeron-ar-rec`,
`aeron-ar-rep`, `aeron-cl-cm` and `aeron-cl-cs-<service id>`:

```sh
pid=$(pgrep -f io.aeron.cluster.ClusteredMediaDriver)
for task in /proc/"$pid"/task/*; do
    case $(cat "$task/comm") in
        aeron-cl-cm) taskset -pc 4 "${task##*/}" ;;
        aeron-ar-rec) taskset -pc 5 "${task##*/}" ;;
    esac
done
```

Every spin or noop thread needs a whole isolated core of its own (see
[Hosts](hosts.md)). On shared cores, keep Aeron's default backoff.

## Small or shared hosts

When the JVM shares its cores, or runs in a container with a few hundred MiB:

- Keep the backoff idle strategies. A spinning thread on a shared core takes
  the core from everything else.
- Serial GC (`-XX:+UseSerialGC`) is the one-core collector; its cost against
  ParallelGC here is **not measured**.
- Set `-Xms` equal to `-Xmx`, and size `-XX:MaxDirectMemorySize`,
  `-XX:MaxMetaspaceSize` and `-XX:ReservedCodeCacheSize`. The container's
  limit must cover all of them, plus thread stacks and native memory.
- Leave `aeron.pre.touch.mapped.memory` off. Every term a pre-touching client
  maps counts against the container's memory limit.

## Options to leave out of older guides

- `-XX:+UseParallelOldGC`: Java 17 and 25 refuse to start with it. Aeron's
  wiki still lists it.
- `-XX:-UseBiasedLocking`: deprecated with a warning on 17. Java 25 refuses
  to start with it.
- `-XX:+ZGenerational`: Java 25 ignores it with a warning, and 27 refuses to
  start. ZGC is now generational only.
- The `sun.misc.Unsafe` deprecation (JEP 471, JEP 498) and
  `--sun-misc-unsafe-memory-access` do not apply. Agrona uses
  `jdk.internal.misc.Unsafe`, which is why the first `--add-opens` is needed.

## Optional: measure first

These options are not in Aeron's sources:

- `-XX:+UseCompactObjectHeaders` (a product option since Java 25,
  [JEP 519](https://openjdk.org/jeps/519)) makes objects smaller.
- `-XX:+PerfDisableSharedMem` stops the JVM writing its hsperfdata file under
  `/tmp`, at the cost that `jps` and `jstat` no longer see it.
- Generational ZGC (`-XX:+UseZGC`) is only worth it for a clustered service
  that allocates, needs sub-millisecond pauses and has cores to spare.
- Huge pages through the JVM (`-XX:+UseTransparentHugePages`, Linux only)
  cover only the heap, which is cold in steady state. Aeron's hot memory is its
  mapped log buffers; size those with `aeron.file.page.size` on hugetlbfs, as
  [Hosts](hosts.md) describes.

## In ergon's tests

The cluster harness (`ergo-aeron-cluster-test-harness`) runs
`$JAVA_HOME/bin/java`, else `java` on `PATH`. A `java` older than 17, or one
that will not run, fails the test with its path and the fix. Every Java
process it starts reads `cluster-test-harness/jvm/test.jvm`, which follows
Aeron's own test settings: checks on, nothing spins.

```text
{{#include ../../../cluster-test-harness/jvm/test.jvm}}
```

```sh
export JAVA_HOME=$(/usr/libexec/java_home -v 25)   # macOS; on Linux, the JDK's directory
just build-aeron-jars
just test-aeron-cluster-harness
```

## Sources

- Aeron 1.54.0:
  [`aeron-samples/scripts/java-common`](https://github.com/aeron-io/aeron/blob/1.54.0/aeron-samples/scripts/java-common),
  the cluster scripts in
  [`aeron-samples/scripts/cluster`](https://github.com/aeron-io/aeron/tree/1.54.0/aeron-samples/scripts/cluster),
  [`low-latency.properties`](https://github.com/aeron-io/aeron/blob/1.54.0/aeron-driver/src/main/resources/low-latency.properties),
  [`build.gradle`](https://github.com/aeron-io/aeron/blob/1.54.0/build.gradle)
  and its CI matrix in
  [`.github/workflows/ci.yml`](https://github.com/aeron-io/aeron/blob/1.54.0/.github/workflows/ci.yml).
- [aeron-io/benchmarks](https://github.com/aeron-io/benchmarks):
  `scripts/run-java` and `scripts/aeron/low-latency-*.properties`.
- [Aeron wiki: Performance Testing](https://github.com/aeron-io/aeron/wiki/Performance-Testing).
- [Gradle's Java compatibility matrix](https://docs.gradle.org/current/userguide/compatibility.html).
- [JEP 471](https://openjdk.org/jeps/471), [JEP 498](https://openjdk.org/jeps/498)
  and [JEP 519](https://openjdk.org/jeps/519).
