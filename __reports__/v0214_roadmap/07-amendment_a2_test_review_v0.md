# Gap Analysis: v0214-sandbox — PR B's added tests, reviewed for tautology, idiom and CONVENTIONS (A2)

## Problem Statement
The user asked for an independent check that PR B's new tests are not tautological, read like the maintainer's own tests, and respect CONVENTIONS.md. Two read-only reviewers (one for the library tests, one for the `hmn` CLI tests) examined `1bb9b22..807cd2f`. They ran mutations in throw-away worktrees and swapped every `#[allow]` that PR B added in test code for `#[expect]` under stable default, stable `--all-features`, `x86_64-unknown-linux-gnu` and 1.88. Most findings come from what the leaf specs mandated, not from implementer error, so no leaf's own gates caught them.

## Evidence (each mutation was run by a reviewer)
| # | Finding | Proof |
|---|---|---|
| 1 | `macos_smoke::process_exists_under_sandbox_profiles` can pass without checking anything | It passes when run inside `sandbox-exec -p "(version 1)(allow default)"` (the skip path), and with `HMN_PE_CHILD=1` inherited (the child role asserts nothing), even with a planted `pid == 0` special case. Run directly, the same mutant fails |
| 2 | The macOS `NoGpuSource` text is only checked for `Metal` | The text `no GPU measurement source available (Metal failed)` passes the whole suite |
| 3 | The `tests/smoke.rs` macOS tightening never runs unsandboxed | `index + 1 >= count` in the Metal arm: `smoke` 5/5 and `macos_smoke` 10/10 pass unsandboxed (Metal answers index 0 before `bounds_check`) |
| 4 | The all-failed closing line is unpinned | Deleting `eprintln!("{ALL_DEVICES_FAILED_LINE}")` fails no test |
| 5 | An unneeded test lint allow | `clippy::expect_used` on the `metal.rs` test module is an unfulfilled expectation on stable default, stable `--all-features` and 1.88 |
| 6 | Stale docs and foreign idiom | The `macos_smoke.rs` and `cli_ps.rs` `//!` headers contradict the files; the ignore reasons say "needs" while every base reason says "requires"; `#[test]` comes before `#[cfg]` twice (0 times at the base, which writes `#[cfg]` first 69 times); the `kinfo` allow has no reason |
| 7 | `if cfg!(windows) { a } else { b }` in 6 tests | `cfg!(` appears 0 times in any `.rs` file at 1bb9b22; the maintainer writes `#[cfg]` |
| 8 | `remedy_outside_sandbox_is_the_macos_compile_time_platform` is tautological | It restates the const's definition. `REMEDY_OUTSIDE_SANDBOX = false` fails it and the `#[cfg]` twin `format_ps_summary_on_macos_…` alike; a wrapper passing `false` fails only the twin. No mutant fails it alone |
| 9 | The two `n_a_cell` alignment tests are redundant, and their stated claim is false | Every mutant they catch (an unpadded last column, widths taken from the first row) is caught by an older test. A widening glyph (`unmeasured`) passes both, against their "no column widens" claim. The ps one copies a 14-line fixture |
| 10 | Duplicate assertions | The first assert of `remedy_text_outside_sandbox_…` equals the seam test byte for byte; `matches("re-run").count() == 1` is implied by the literal `assert_eq!` before it |
| 11 | `accept()` returns a `&'static str` both callers discard | Read from the code |

Accepted residuals, stated in the PR body rather than fixed: the `ps_exit_code(…, failed)` call site needs two GPUs to drive end to end, and `process_sample` passing `REMEDY_OUTSIDE_SANDBOX` to the growth hint is stderr of a live run. Each is pinned by a `grep -c` gate only. The kinfo portable tests prove self-consistency only; the live anchor test closes that gap (it caught consistent wrong offsets 244 and 44) and runs on arm64 in CI and on x86_64 by the Rosetta command. `P_COMM_SIZE` ≥ 16 is not testable, since parsing cuts at the first NUL.

## Root Cause
The leaf specs fixed test shapes to satisfy count gates (`--list` filters, `grep -c`) and the campaign's CLIPPY IN TESTS rule, which is stricter than the maintainer's practice: he allows `unwrap_used` and `panic` locally with a reason. Those choices were reviewed for gate integrity, not for test strength or house idiom.

## Impact Assessment
- Scope: test code, test comments and test docs only, in `src/error.rs`, `src/gpu/{mod,metal,kinfo}.rs`, `src/bin/hmn/{format,ps,watch}.rs` (`#[cfg(test)]` modules), `tests/{smoke,macos_smoke,cli_ps}.rs`. No production line changes, so part1_close's release-build captures stay valid.
- Downstream contracts kept: `accept(label, code, stderr, expected)` and its branch line (PR C's `ps_watch_unreadable`, part1_close's CI record); the seam test `remedy_outside_sandbox_for_names_is_the_bare_macos_remedy`; the test names `no_gpu_source_display_names_the_backends_of_the_platform` and `gpu_processes_returns_result_or_no_gpu_source` (PR C's `gpu_process_listing`); `macos_smoke --ignored` stays at 3 tests (part1_close). One sentence of `ps_watch_unreadable.md` that listed the deleted tautology is updated in the same roadmap commit.
- Historical counts that move: spill_cell_na's Step 2 check counted 21 tests for its filter; with `format_ps_table_n_a_cell_keeps_the_columns_aligned` deleted, the folded commit counts 20. That gate was met at merge, and the change is recorded here.

## Proposed Solution
One leaf, `part1/test_review_b.md`, with five steps, one per PR B leaf commit that introduced the tests. Each step's commit is folded into its target commit by the coordinator (a non-interactive `git rebase` with `fixup` lines), every rewritten commit is re-run through the full gate set, and `v0214-part1` is force-pushed with `--force-with-lease`, with the user's yes. No PR is open yet, so no reviewer loses context. The leaf is test-only and mechanical, so no verifier; the gates carry the reviewers' mutations as before → after values.

## Recommendations
1. Approved by the user on 2026-10-04: "All of 1–11", "Fold into leaf commits, force-push".
2. Rebase `task/part1_close` onto the rewritten tip afterwards; its Step 2 edits the same `tests/smoke.rs` comment as Step 1 here.
3. For PR C: apply the same review to `kinfo_enumeration`, `gpu_process_listing` and `ps_watch_unreadable` tests before their merge, and reconsider the CLIPPY IN TESTS rule's ban on `unwrap`/`panic` in tests, which the maintainer does not follow.
