# T-0046: Costs That Grow With the History

Status: done.
Dependencies: none.

## Goal

Reduce history-dependent work that delays reports and commands as saved sessions grow.

## Scope

Improve report allocation and period queries, cache recorded activity usage, and reduce snapshot
passes without changing the schema. Record remaining costs in ADR-0042.

## Plan

Measure synthetic histories, change the costly paths, and verify results and query plans.

## Acceptance criteria

- [x] Reports allocate each session only to overlapping days and period queries use the end index.
- [x] Catalog usage is reused and updated or invalidated by history writes.
- [x] Snapshot aggregation uses one pass and stopped activity changes check review directly.
- [x] Record measurements, remaining costs and unverified behavior.

## Checks

- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo test`: 248 passed. Three new tests: `day_rows_of_a_long_period_match_allocation_over_every_day`
  compares a 13-month period across two DST changes with allocation over every day;
  `period_queries_read_through_the_end_index_instead_of_the_whole_history` asserts both period
  query plans use `focus_sessions_ended`; `known_usage_matches_a_fresh_history_scan_after_every_kind_of_write`
  covers start, switch, stop, running-activity edit, manual add and delete, project deletion, and
  proves the set is reused by a change made outside the store.
- Cross-review by Codex GPT-6.1 Sol (high): no correctness findings.
- Benchmark before (unchanged commit `d02c9b9`, same example) and after, as below.
- Version 0.26.4 in Cargo.toml and Cargo.lock.

Median milliseconds, before -> after, on the same machine with synthetic histories:

| History | Whole period | Today |
| --- | --- | --- |
| 1 year x 20/day (7,300 sessions) | 221 -> 172 | 91 -> 5 |
| 3 years x 20/day | 494 -> 177 | 82 -> 7 |
| 5 years x 20/day | 987 -> 189 | 85 -> 155 |
| 10 years x 20/day (73,000 sessions) | 3,303 -> 335 | 76 -> 3 |
| 10 years x 50/day (182,500 sessions) | 8,346 -> 719 | 211 -> 4 |

The machine was loaded by other work; single numbers vary by two to three times, while trends
are stable. The 155 ms today result was one noisy run; other today values were 3 to 7 ms.

Unverified: the running application with a long real history, and the report window with a
multi-year period in the real UI. By day still lays out every day each frame; this was not measured.

- `scripts/build.ps1` built both 0.26.4 EXEs and verified `dist/AttentionFocus-0.26.4`, and
  `scripts/verify-release.ps1` passed: allowlist, source fingerprint, hashes, manifest, icons and
  file metadata. The notices did not change. The guide changed only in its version, so its pages
  were not reviewed again. Two earlier build attempts were stopped by the environment for low
  memory, before they wrote any output.

## Handoff

The owner decides whether to publish 0.26.4 and whether to pursue T-0047. One aggregate pass
per snapshot and the review check in `begin` remain linear; defer a partial state index to the
next schema change, with backup and upgrade tests. Opening reads usage once.
