
### 2026-07-17 final: wrap_trusted 5-run median — decode/full_message 1.030

NgDecoder `wrap_trusted` (uses standard `read_bytes`, skips redundant
`pos + dim_size > buf.len()` check when the entry tail cache is warm) landed
alongside the existing tail cache + unsafe elision. All 18 sbe test binaries
green, allocation 7/7, golden regenerated.

5-run medians (2026-07-17, post-wrap_trusted):
ErgoSBE consuming **11.226 ns**, Aeron **10.904 ns** — ratio **1.030**
(was 1.151 before cache, 1.034 after cache+elision, 1.030 after
cache+elision+wrap_trusted).

The 0.32ns residual is not attributable to any remaining codec walk or
bounds check — every segment individually ≤ Aeron (fuel 3.46 vs 4.36,
perf-cumulative 7.91 vs 8.08, vardata chain 4.95). The root cause is
LLVM's composed-function register allocation and inter-stage type boundaries
preventing the same optimisation each segment enjoys in isolation.

encode/throughput_10k (1.135) has not been further improved beyond the
single-bounds-check wrap and DSE-proof harness (both previously committed).
