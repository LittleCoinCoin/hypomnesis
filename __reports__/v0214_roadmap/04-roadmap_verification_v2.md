# Roadmap verification — v0214-sandbox, round 2

| Verifier | Scope | Confirmed blocker/major | Notes |
|---|---|---|---|
| V-AB2 | PR A, PR B gates | 0 | every round-1 row APPLIED; it prototyped the `process_exists_kinfo` 1–3 and `remedy_macos` 1–2 squash groups, and both end green on the full gate set, including 1.88 clippy and test; 5 minor |
| V-C2 | PR C gates | 2 majors | every round-1 row APPLIED; it prototyped `kinfo_enumeration` 1–2, and the self-row triples match the leaves under every profile; the `entries.is_empty()` mutant is caught by gates 21–22 |
| V-SEAM2 | cross-leaf contracts | 6 conflicts | the remedy, device-failure, listing, harness and CI contracts AGREE |

The PR C major on `clippy::panic` and V-SEAM2's C1 are the same defect, so round 2 has 7 distinct
blocker/major defects. Round 1 had 14. Every one below is **FIX** unless marked otherwise.

## Global (coordinator, campaign README)

| ID | Source | Fix |
|---|---|---|
| R2-G1 | V-SEAM2 C6 | Leaves cite another leaf's gate by its quoted text, never by number. Two numbering schemes (file line and bullet count) were in use. |
| R2-G2 | V-C2 2, V-AB2 3, V-C2 P3 | CLIPPY IN TESTS: `Cargo.toml` denies `clippy::panic`, `unwrap_used`, `expect_used` and `wildcard_enum_match_arm`, and CI runs `clippy --all-targets`. Test code uses a `Cell<bool>` or `AtomicBool` flag instead of a panicking closure, uses explicit match arms (no `_`), and writes a per-fn `#[allow(clippy::expect_used)] // test-only` where it must, as `tests/macos_smoke.rs` does. |
| R2-G3 | V-AB2 P1 | The shared gate set adds `cargo +1.88 clippy --locked --all-targets --all-features -- -D warnings` and `cargo +1.88 test --locked --all-features`, matching CI's 1.88 leg. |
| R2-G4 | V-AB2 P2 | The part1 README says `ps_failed_devices`' CI gate is checked off only after the draft-PR push, which follows the merge of the five code leaves. That leaf is marked `done` at that point. Its pre-condition quote of `docs_pr`'s gate is corrected to "PR A is open and CI is green". |

## PR A — `docs_pr`

| ID | Source | Fix |
|---|---|---|
| R2-A1 | V-SEAM2 C5 | At release, `__reports__` links in `docs/` and `ROADMAP.md` become a plain mention of the file name plus the commit SHA that last held it. No `blob/<sha>/__reports__/…` URLs, because `part2_close`'s grep forbids the string. The sentence attaches to the *At release* bullet about `__reports__/`, the third bullet, and the gate token is reworded to match. |
| R2-A2 | V-AB2 4 | The R01 sentence on the x86_64 run names no campaign leaf: "the x86_64 unit-test run is a v0.2.14 PR B check; until it passes…". |

## PR B

| ID | Leaf | Source | Fix |
|---|---|---|---|
| R2-B1 | `ps_failed_devices` | V-AB2 1, 5; V-C2 1 | `\| wc -c \| tr -d ' '`. Spell out each sandboxed variant's `bash -c` form. **Seam:** declare `device_query_failure_line` as `pub(crate)` so PR C can test the skip line, and cite that in Deliverables. |
| R2-B2 | `process_exists_kinfo` | V-AB2 2, 3 | Step 3 Deliverables say "the Step 1 `allow(dead_code)` removed; the two `missing_const_for_fn` allows in `kinfo.rs` removed". Step 1 item 5 adds `#[allow(clippy::expect_used)] // test-only` for the `macos_smoke` test (R2-G2). |
| R2-B3 | `part1_close` | V-SEAM2 C3, C4 | Step 4 item 1 drops "in the `hmn` column (Step 5)". The Claude Code PR B result goes only in the sentence under the table. Add a fourth "pending, PR C" Verification row for the App Sandbox PR C form. Gates count 4. |

## PR C

| ID | Leaf | Source | Fix |
|---|---|---|---|
| R2-C1 | `kinfo_enumeration` | V-C2 2, 7; V-SEAM2 C1, 3 | The panicking closures become flag closures, with a test asserting the flag stayed false (R2-G2). Step 2 items on `process_gpu_info` and enumerate say "explicit arms, no `_`". Reword "Every test must fail against its stub" to name the one that passes by construction. Gate 25's `allow(dead_code)` guard counts `src/gpu/mod.rs` and `src/gpu/metal.rs`, where the attribute actually sat, not only `kinfo.rs`. |
| R2-C2 | `gpu_process_listing` | V-SEAM2 C2, V-C2 P3 | Extend the `error.rs` `mod tests` that `metal_bounds_check` created; never add a second one (E0428). Add a pre-condition quoting `metal_bounds_check`'s gate. `tests/macos_sandbox.rs` carries the R2-G2 allows. |
| R2-C3 | `ps_watch_unreadable` | V-C2 1, 4; V-SEAM2 minor | The skip-line assertion moves to `ps::tests`. It keeps a `failure_detail_` name so the count of 3 holds, and calls `pub(crate) device_query_failure_line` (R2-B1). `accept` keeps a third, separately labelled branch, `skipped-device-nogpu`, for a skip line carrying the `NoGpuSource` text. A macos-latest VM whose ledger template does not resolve legitimately gives that, and PR B accepted it. `part2_close` records which branch fired on PR C's CI, as `part1_close` does for PR B. The pre-condition also cites `remedy_macos`'s seam gate by quoted text. |
| R2-C4 | `part2_close` | V-SEAM2 C4, C5; V-C2 5, 6 | Step 3 *creates* the S0, L, Rosetta and architecture Verification rows, rather than "keeping" them. It fills the four "pending, PR C" rows, including the App Sandbox one, so the gate goes 4 → 0. Step 5's dead links follow R2-A1, a plain mention plus SHA. Hashes are recorded after the final rebase onto `main` and after the red squash, or paths alone are named. Add the gate `grep -c 'refuses .proc_listpids' README.md` 1 → 0. Record the PR C CI branch (R2-C3). Keep the token `the sandbox decides` verbatim in item 9. |

## Rejected

- **V-C2 P8**, the `PartialEq` derive making a field allow unnecessary: V-AB2 already measured it
  as holding ("`KinfoRecord::comm` triggers no warning with PartialEq derived"). There is nothing to
  change.

## Next

Two fixers apply these items: one for PR A and PR B, one for PR C. The coordinator applies R2-G1–G4.
Round 3 is a focused check on the changed text only. It covers the gates touched by R2 rows, a
clippy `--all-targets` prototype of `kinfo_enumeration` Steps 1–2 and `ps_watch_unreadable` Step 1,
and the seams C2–C5.
