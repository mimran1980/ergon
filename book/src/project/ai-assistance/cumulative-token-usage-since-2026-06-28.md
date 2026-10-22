# Cumulative token usage (since 2026-06-28)

Per-model figures for Claude Code, Codex, and Grok Build CLI. The other
machine's per-model high-water mark — every `ccusage` snapshot committed to
this file (2026-08-03, 2026-08-07, 2026-08-13, 2026-08-15, 2026-08-19,
2026-08-20, 2026-08-27, 2026-09-07, 2026-09-13 and 2026-09-14, plus the
2026-10-13 run) — is added to this MacBook's local `ccusage` 20.0.40 reads on
2026-10-21. A later `npx ccusage@latest --since 2026-06-01 -b` that day raised
only the Grok row. It did not replace the other machine, and it did not drop
Codex usage from before 2026-06-01. The two ledgers are summed. A `ccusage`
run on either machine alone will not match this table.

| Model | Input | Output | Cache Create | Cache Read | Reasoning Output | Total Tokens | Cost (USD) |
|---|---:|---:|---:|---:|---:|---:|---:|
| claude-fable-5 | 403,485 | 1,349,254 | 12,230,126 | 487,557,778 | — | 501,540,643 | $802.28 |
| claude-haiku-4-5-20251001 | 936 | 31,021 | 203,279 | 10,487,325 | — | 10,722,561 | $1.56 |
| claude-opus-4-8 | 189,029 | 553,025 | 4,804,415 | 97,619,868 | — | 103,166,337 | $99.09 |
| claude-opus-5 | 13,757,961 | 4,705,886 | 36,674,520 | 3,652,158,264 | — | 3,707,296,631 | $2,360.26 |
| claude-opus-5-5 | 220,976,251 | 113,604,569 | 593,122,783 | 27,123,459,935 | — | 28,051,163,538 | $11,887.75 |
| claude-sonnet-4-6 | 1,406 | 233,671 | 986,400 | 34,920,711 | — | 36,142,188 | $19.90 |
| claude-sonnet-5 | 88,990 | 2,988,394 | 31,267,091 | 2,340,454,308 | — | 2,374,798,783 | $622.06 |
| claude-sonnet-5-5 | 11,512 | 5,102,015 | 32,656,109 | 834,727,182 | — | 872,496,818 | $221.22 |
| deepseek-flash | 8,991,713 | 5,070,512 | — | 3,202,997,632 | — | 3,217,059,857 | $28.00 |
| deepseek-v4-flash | 52,791,367 | 17,446,045 | — | 2,734,161,536 | — | 2,802,078,889 | $35.00 |
| deepseek-v4-pro | 85,359,087 | 21,733,136 | — | 24,546,493,056 | — | 24,653,585,279 | $551.30 |
| glm-4.7 | 6,949,701 | 760,380 | — | 268,436,096 | — | 276,146,177 | $35.37 |
| glm-5.2 | 27,277,618 | 3,460,625 | — | 2,274,775,808 | — | 2,305,514,051 | $644.86 |
| gpt-5.5 | 7,380,642 | 366,392 | — | 69,651,456 | 105,368 | 77,503,858 | $92.47 |
| gpt-5.6-luna | 1,754,305 | 188,525 | — | 54,281,984 | 105,116 | 56,329,930 | $36.18 |
| gpt-5.6-sol | 60,151,783 | 5,239,880 | — | 1,520,516,224 | 2,617,722 | 1,588,525,609 | $1,011.70 |
| gpt-5.6-terra | 676,005 | 34,650 | — | 7,455,232 | 16,229 | 8,182,116 | $4.33 |
| gpt-6-astra | 9,061,594 | 681,360 | — | 140,212,992 | 217,983 | 150,173,929 | $236.08 |
| gpt-6.1-sol | 18,651,728 | 2,136,766 | — | 522,706,816 | 976,793 | 544,472,103 | $146.59 |
| grok-4.5-build | 10,030,758 | 881,349 | — | 177,071,360 | — | 187,983,467 | $114.67 |
| grok-4.6-build | 44,849,892 | 3,100,656 | — | 812,446,592 | — | 860,397,140 | $293.80 |
| grok-4.7-build | 39,298,841 | 5,592,936 | — | 621,799,168 | — | 666,690,945 | $166.51 |
| mimo-v2.6-pro | 62,028,500 | 1,093,150 | — | 1,875,607,680 | — | 1,938,729,330 | $34.69 |
| gpt-5.3-codex | 22,573,884 | 2,873,034 | — | 786,800,512 | 1,263,964 | 813,511,394 | $217.63 |
| gpt-5-codex | 9,351,684 | 988,119 | — | 204,395,904 | 689,664 | 215,425,371 | $45.08 |
| gpt-5.1-codex-max | 2,116,543 | 122,477 | — | 45,743,360 | 46,976 | 48,029,356 | $11.63 |
| gpt-5.4 | 1,517,763 | 54,121 | — | 14,817,280 | 24,110 | 16,413,274 | $8.09 |
| gpt-5 | 701,637 | 79,205 | — | 8,801,920 | 51,776 | 9,634,538 | $2.77 |
| mimo-v2.6-flash | 359,723 | 15,224 | — | 1,104,448 | — | 1,479,395 | $0.06 |
| gpt-5.4-mini | 122,102 | 4,288 | — | 434,560 | 2,399 | 563,349 | $0.20 |
| **Total** | **707,426,440** | **200,490,665** | **711,944,723** | **74,472,096,987** | **6,118,100** | **76,095,756,856** | **$19,731.14** |

**Notes:**

- The other machine's baseline is a **high-water mark**, not a single run's output. `ccusage` keeps no
  ledger: it recomputes totals by re-reading local session transcripts, and
  Claude Code deletes transcripts older than `cleanupPeriodDays`. A
  `--since 20260628` query therefore *loses* its earliest days as time passes,
  so a later snapshot can report **fewer** tokens than an earlier one. That
  baseline is the per-model, per-column maximum across all eleven snapshots;
  since the tool can only under-report, that maximum is the best available
  lower bound. This MacBook's 2026-10-21 read is added on top of that baseline.
  A second read the same day, filtered to `--since 2026-06-01`, is merged the
  same way: where it was higher, that column replaced this machine's earlier
  figure; where it was lower, the earlier figure stays. The transcripts are
  different files, so each published row is the sum.
- The 2026-10-13 run alone reported **57,084,644,123** tokens. Merging the
  snapshots recovered from this file's git history adds **2,589,115,055**
  tokens the current run can no longer see. A later `ccusage` on one machine
  is added the same way. It does not replace this table.
- In the 2026-10-13 snapshot: `claude-opus-5-5`, `claude-sonnet-5-5`, `deepseek-flash`,
  `gpt-6.1-sol` and `grok-4.7-build`. `claude-opus-5`, `claude-sonnet-5`,
  `deepseek-v4-pro`, `gpt-6-astra` and `grok-4.6-build` set new total maxima.
  Every other model's maximum is held by an earlier snapshot;
  `grok-4.5-build` appears only in earlier snapshots.
- Grok Build CLI rows come from `ccusage grok`, which extracts them from the
  Grok CLI session logs.
- Antigravity and Gemini CLI are not in the table. The other machine had no
  `observer` sample, and its usage of those tools is still absent. This
  MacBook's `ccusage antigravity` reports **290,048,735** tokens and **$83.86**
  (2026-06-20 to 2026-07-05). `ccusage gemini` reports **553,921** tokens and
  **$0.56** (November 2025, December 2025, and February 2026). Those reads are
  recorded here and were not added to the rows above.
- Hermes and OpenCode are excluded — automation, not development work on this
  project. This MacBook had no Hermes transcripts. Its OpenCode total is
  178,124,443 tokens and stays out of the table.
- Codex reports cost per day rather than per model. On days that used more
  than one model, the day's cost is split between them in proportion to their
  token share; token counts themselves are exact per-model figures from the
  tool. On the 2026-10-21 MacBook read, Codex `totalTokens` omits reasoning.
  The day's cost was split on that `totalTokens`. Reasoning is then included
  in the Total Tokens column.
- All costs are at enterprise/pay-as-you-go API rates observed by each tool.
  Subscription fees (Claude, OpenAI, Grok) are not included.
- Sanity checks: the per-model column sums for the 2026-10-13 run reconcile
  exactly against each tool's own reported totals (53,825,172,584 Claude Code +
  2,019,689,986 Codex including 3,537,371 reasoning output + 1,239,781,553
  Grok = 57,084,644,123), and the grand total is monotonic against the
  previously committed 25,853,122,297.
- 2026-10-21 MacBook check, `ccusage` 20.0.40 with cached offline prices.
  The first read was the full local history: Claude Code 14,674,098,035 +
  Codex 1,458,109,013 + Codex reasoning 2,484,098 + Grok 266,056,280 =
  16,400,747,426. Claude Code covered 2026-06-26 to 2026-10-21, Codex
  2025-09-13 to 2026-10-21 (1,154,747,421 of those tokens are before
  2026-06-28), and Grok 2026-10-14 to 2026-10-21. A later read the same day,
  `npx ccusage@latest --since 2026-06-01 -b`, reported every detected agent:
  15,583,155,859 tokens and $5,108.42. Antigravity was 290,048,735 tokens and
  $83.86, the figure already recorded, and stays out of this table. Gemini CLI
  and OpenCode had no rows in that window. Claude Code's token total was
  unchanged. Codex token columns for models with usage on or after 2026-06-01
  matched the first read, including their reasoning. Codex before 2026-06-01
  is outside that window, so those rows stay at the first read (`gpt-5.3-codex`,
  `gpt-5-codex`, `gpt-5.1-codex-max`, `gpt-5.4`, `gpt-5`, `gpt-5.4-mini`, and
  the earlier part of `gpt-5.5`). The unified report also repriced those same
  Codex tokens; that reprice was not applied, so the published Codex costs
  remain the daily split from the first read. Grok grew by 573,183 input,
  250,829 output and 20,426,240 cache read (21,250,252 tokens; unrounded cost
  from $57.056 to $61.702). The MacBook ledger in this table is therefore
  Claude Code 14,674,098,035 + Codex 1,458,109,013 + Codex reasoning 2,484,098
  + Grok 287,306,532 = 16,421,997,678. Added to the other machine's
  59,673,759,178 this is **76,095,756,856**. Unrounded costs are Claude Code
  $4,737.668, Codex $537.250 and Grok $61.702, together **$5,336.62**. The
  previous cost total was $14,394.52; combined **$19,731.14**.
  `deepseek-v4-flash`'s Total Tokens cell remains 2,320,059 below the sum of
  its own columns, as previously published.
