# Cumulative token usage (since 2026-06-28)

Per-model figures for Claude Code, Codex, and Grok Build CLI, summed over
two machines: a Mac mini (called "the other machine" in the older notes
below) and a MacBook. Each machine's figure is a per-model, per-column
high-water mark of its own `ccusage` reads. Both machines were last read on
2026-11-05. The Mac mini's earlier mark comes from every `ccusage` snapshot
committed to this file (2026-08-03, 2026-08-07, 2026-08-13, 2026-08-15,
2026-08-19, 2026-08-20, 2026-08-27, 2026-09-07, 2026-09-13 and 2026-09-14,
plus the 2026-10-13 run); the MacBook's from its reads on 2026-10-21. Claude
Code transcripts from 2026-10-13 to 2026-11-03 exist on both machines,
because the MacBook's were copied to the Mac mini on 2026-11-03; those days
count once. A `ccusage` run on either machine alone will not match this
table.

| Model | Input | Output | Cache Create | Cache Read | Reasoning Output | Total Tokens | Cost (USD) |
|---|---:|---:|---:|---:|---:|---:|---:|
| claude-fable-5 | 403,485 | 1,349,254 | 12,230,126 | 487,557,778 | — | 501,540,643 | $802.28 |
| claude-haiku-4-5-20251001 | 936 | 31,021 | 203,279 | 10,487,325 | — | 10,722,561 | $1.56 |
| claude-haiku-5-5 | 12 | 21,757 | 165,636 | 152,101 | — | 339,506 | $0.08 |
| claude-opus-4-8 | 189,031 | 554,780 | 4,981,359 | 97,619,868 | — | 103,345,038 | $100.90 |
| claude-opus-5 | 13,758,229 | 4,833,901 | 37,960,920 | 3,736,999,202 | — | 3,793,552,252 | $2,418.75 |
| claude-opus-5-5 | 251,055,503 | 148,796,765 | 877,700,171 | 36,319,018,461 | — | 37,596,570,900 | $16,029.01 |
| claude-sonnet-4-6 | 1,406 | 233,671 | 986,400 | 34,920,711 | — | 36,142,188 | $19.90 |
| claude-sonnet-5 | 88,994 | 2,990,761 | 31,408,720 | 2,340,474,205 | — | 2,374,962,680 | $622.44 |
| claude-sonnet-5-5 | 17,266 | 7,083,348 | 55,569,040 | 1,291,485,363 | — | 1,354,155,017 | $345.70 |
| deepseek-flash | 8,991,713 | 5,070,512 | — | 3,202,997,632 | — | 3,217,059,857 | $28.00 |
| deepseek-v4-flash | 52,791,367 | 17,446,045 | — | 2,734,161,536 | — | 2,802,078,889 | $35.00 |
| deepseek-v4-pro | 85,359,087 | 21,733,136 | — | 24,546,493,056 | — | 24,653,585,279 | $551.30 |
| glm-4.7 | 6,949,701 | 760,380 | — | 268,436,096 | — | 276,146,177 | $35.37 |
| glm-5.2 | 27,277,618 | 3,460,625 | — | 2,274,775,808 | — | 2,305,514,051 | $644.86 |
| gpt-5.5 | 7,380,642 | 366,392 | — | 69,651,456 | 105,368 | 77,503,858 | $92.47 |
| gpt-5.6-luna | 1,754,305 | 188,525 | — | 54,281,984 | 105,116 | 56,329,930 | $36.18 |
| gpt-5.6-sol | 60,170,156 | 5,240,538 | — | 1,520,615,040 | 2,617,722 | 1,588,643,456 | $1,011.83 |
| gpt-5.6-terra | 693,820 | 34,722 | — | 7,463,168 | 16,229 | 8,207,939 | $4.37 |
| gpt-6-astra | 10,296,875 | 740,421 | — | 153,900,160 | 217,983 | 165,155,439 | $262.68 |
| gpt-6.1-sol | 29,423,415 | 3,171,384 | — | 808,357,760 | 976,793 | 841,929,352 | $224.99 |
| grok-4.5-build | 10,030,758 | 881,349 | — | 177,071,360 | — | 187,983,467 | $114.67 |
| grok-4.6-build | 44,849,892 | 3,100,656 | — | 812,446,592 | — | 860,397,140 | $293.80 |
| grok-4.7-build | 55,733,853 | 8,299,594 | — | 962,121,472 | — | 1,026,154,919 | $244.67 |
| mimo-v2.6-pro | 63,865,972 | 1,135,818 | — | 1,930,584,704 | — | 1,995,586,494 | $66.97 |
| gpt-5.3-codex | 22,573,884 | 2,873,034 | — | 786,800,512 | 1,263,964 | 813,511,394 | $217.63 |
| gpt-5-codex | 9,351,684 | 988,119 | — | 204,395,904 | 689,664 | 215,425,371 | $45.08 |
| gpt-5.1-codex-max | 2,116,543 | 122,477 | — | 45,743,360 | 46,976 | 48,029,356 | $11.63 |
| gpt-5.4 | 1,517,763 | 54,121 | — | 14,817,280 | 24,110 | 16,413,274 | $8.09 |
| gpt-5 | 701,637 | 79,205 | — | 8,801,920 | 51,776 | 9,634,538 | $2.77 |
| mimo-v2.6-flash | 1,197,143 | 24,099 | — | 41,376,064 | — | 42,597,306 | $0.75 |
| gpt-5.4-mini | 122,102 | 4,288 | — | 434,560 | 2,399 | 563,349 | $0.20 |
| **Total** | **768,664,792** | **241,670,698** | **1,021,205,651** | **84,944,442,438** | **6,118,100** | **86,979,781,620** | **$24,273.94** |

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
- Hermes and OpenCode are excluded: automation, not development work on this
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
- 2026-11-05 reads: `npx ccusage@latest daily -b` (20.0.48) on the Mac mini,
  and the same unified daily report on the MacBook. The MacBook report's
  rows reconcile exactly with its own Total row (23,931,907,711 tokens, all
  agents).
- The copy. 1,518 Claude Code transcript files on the Mac mini were created
  at 11:31 on 2026-11-03 (UTC+8): the MacBook's sessions active from 13:02
  on 2026-10-13. From 2026-10-14 to 2026-11-02 the Mac mini's Claude Code
  usage equals the MacBook's per day and model, token for token, except
  2026-10-15 and 2026-10-27, where the Mac mini holds slightly less. The
  Mac mini had no Codex or Grok usage in that window, and neither tool's
  logs were copied. Those 21,770,555,386 Claude Code tokens count once, in
  the MacBook's figures. The MacBook's Claude rows for 2026-10-13 and
  2026-11-03 are subtracted from the Mac mini's read of those days, with
  their cost. The whole 2026-10-13 row is subtracted although only its
  afternoon was copied, so up to about 43 million tokens of that day may be
  missing. Like transcript cleanup, this can only under-report.
- MacBook: per model and column, the larger of its 2026-10-21 figures and
  the new read up to 2026-10-22, plus the new read from 2026-10-23. The
  2026-10-21 read already held part of 2026-10-22, so the split falls there.
  The MacBook adds 8,286,299,912 tokens and $3,422.62.
- Mac mini: the larger of its 2026-10-13 high-water mark and the new read up
  to 2026-10-13, plus the new read from 2026-10-14 without the copied days.
  It adds 2,596,671,615 tokens and $1,120.18.
- Cost: published costs stay as they are. Added tokens carry the cost the
  new read gives them; where a read up to a split date exceeds the earlier
  figure, the excess is priced at that segment's average rate. The new
  reads price `mimo-v2.6-pro` at about $0.57 per million tokens, against
  about $0.018 in the 2026-10-21 read; the earlier tokens keep their earlier
  cost.
- The new reads have no reasoning column. Reasoning output stays at its
  earlier figures, and a changed row's Total Tokens is the sum of its
  columns, which carries 1,053,237 earlier reasoning tokens of `gpt-6-astra`
  and `gpt-6.1-sol` into Total.
- Not added, as before: on the Mac mini, Gemini CLI 375,413,088 tokens
  ($183.59), Hermes 311,780,314, OpenCode 246,593,840, Antigravity
  44,554,114 and ZCode 22,091,463; on the MacBook, Antigravity 290,048,735
  (unchanged) and OpenCode 62,841,911 (2026-10-23 and 2026-10-24).
- The grand total rises from 76,095,756,856 to **86,979,781,620** tokens and
  the cost from $19,731.14 to **$24,273.94**.
