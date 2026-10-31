# UDP `rx_batch` sweep, omarchy, 2026-10-30

How the receive batch size trades the first datagram of a burst against the
last. omarchy: Intel i5-8500T, 6 cores, Linux 7.2.8, no isolated CPUs, a
day-long recording running on the other cores. Relative numbers only: not a
latency platform. ergon tree: `38dbdd1f` (rx_batch set per arm).

Method (`udp_batch_size.rs.txt` as `src/main.rs`, `Cargo.toml.txt`): two
connected loopback sockets in one `epoll-spin` driver (`datagram_burst=64`,
so the fairness cap never splits a burst), pinned to CPU 3 (`taskset`),
under `flock ~/ergon-work/.bench.lock`. Per burst: queue N datagrams and
`flush` (loopback: all N are in the receiver's queue when it returns), then
start the clock and poll until the handler has seen the last one. The
handler stamps the first and the last datagram (`Instant`). 5,000 warm-up
and 50,000 timed bursts per point; three rounds. Values: range over the
rounds, ns.

## N = 32, 64 bytes

| rx_batch | first p50 / p99 / p99.9 / max | last p50 / p99 / p99.9 / max | calls |
|---|---|---|---|
| 1 | 2,021–2,047 / 2,163–2,202 / 4,649–4,851 / 6,786–20,923 | 34,447–34,488 / 37,585–37,620 / 38,738–38,898 / 47,511–76,855 | 33 |
| 4 | 3,134–3,152 / 3,297–3,392 / 6,152–6,371 / 7,868–16,022 | 18,397–18,441 / 21,128–21,196 / 22,246–22,601 / 29,447–35,109 | 9 |
| 8 | 4,552–4,590 / 4,737–4,904 / 7,707–7,861 / 9,324–12,454 | 15,473–15,521 / 18,090–18,165 / 19,096–19,658 / 20,369–24,901 | 5 |
| 16 | 7,369–7,413 / 7,678–7,815 / 10,587–10,841 / 12,227–55,450 | 13,989–14,036 / 16,536–16,655 / 17,557–18,021 / 19,367–105,594 | 3 |
| 32 | 13,058–13,094 / 15,609–15,638 / 16,549–16,683 / 18,012–25,869 | 13,234–13,272 / 15,790–15,822 / 16,735–16,866 / 18,188–26,060 | 2 |

## N = 32, 1,024 bytes

| rx_batch | first p50 / p99 / max | last p50 / p99 / max |
|---|---|---|
| 1 | 2,163–2,198 / 2,263–2,296 / 13,408–53,634 | 35,382–35,576 / 38,706–38,828 / 48,982–127,771 |
| 4 | 3,369–3,396 / 3,499–3,526 / 7,946–14,658 | 19,307–19,432 / 22,179–22,266 / 31,431–37,456 |
| 8 | 4,927–4,982 / 5,099–5,172 / 11,309–25,025 | 16,448–16,733 / 19,349–19,572 / 23,328–36,891 |
| 16 | 8,006–8,076 / 8,320–8,364 / 13,300–26,225 | 15,122–15,287 / 17,787–17,944 / 26,821–33,497 |
| 32 | 14,168–14,390 / 16,914–17,117 / 23,625–58,488 | 14,348–14,573 / 17,096–17,303 / 23,799–95,903 |

## N = 8, 64 bytes

| rx_batch | first p50 / p99 / max | last p50 / p99 / max |
|---|---|---|
| 1 | 1,949–1,970 / 2,035–2,149 / 5,678–6,811 | 9,317–9,345 / 9,627–10,563 / 14,147–19,181 |
| 4 | 3,036–3,046 / 3,131–3,181 / 8,766–13,158 | 5,308–5,315 / 5,441–5,556 / 11,046–15,507 |
| 8 | 4,493–4,500 / 4,628–4,648 / 9,035–26,252 | 4,551–4,559 / 4,687–4,705 / 9,091–26,312 |
| 16 | 4,647–4,649 / 4,788–4,818 / 9,432–19,674 | 4,704–4,707 / 4,849–4,876 / 9,490–19,734 |
| 32 | 4,703–4,724 / 4,855–4,880 / 9,576–19,839 | 4,760–4,783 / 4,913–4,940 / 9,636–19,898 |

## Reading

- The first datagram waits for its batch to be copied: about 0.35 µs per
  extra datagram (2.0 µs at 1, 13.1 at 32). Each datagram read in a batch
  instead of its own call saves about 0.7 µs (last: 34.5 µs at 1, 13.2 at
  32).
- 8 keeps the first datagram within 2.6 µs of the unbatched read and the
  last within 2.3 µs of the best. The default stays 16: omarchy is a
  desktop without isolated CPUs, so the choice waits for a tuned cloud run
  (sizes 8 to 128). A burst no larger
  than the batch (N = 8) costs the same at 8, 16 and 32.
- 1,024-byte payloads add under 0.05 µs a datagram.
- A full batch takes one more call to find the queue empty (N = 8 at
  `rx_batch` 8: 2 calls); a short one does not.
- Maxima (up to 128 µs) are not attributed: CPU 3 is not isolated, takes
  interrupts and shares the host with the recording; p99.9 stays within
  about 3 µs of p99 everywhere. The run also logged ergon-io's T19 host
  check (about 1,700 minor page faults per check on the loop thread); each
  point builds a new driver and result vectors, so these are setup faults,
  not timed ones, but they are not separated.
