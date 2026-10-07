# Audit R01 item 6: the CLI (`ps.rs`, `format.rs`, `watch.rs`, `main.rs`, `tests/cli_ps.rs`, CHANGELOG)

Read-only analysis of `git diff b716087..HEAD` on `v0214-part2`. Nothing was built.

## Summary

The user-facing behaviour is what the maintainer and R01 asked for, with one small extension. It covers
the remedy wording, `--pid` applied to denied PIDs, `--exit-status` 2 when blind, one notice per denied
explicit PID in `watch`, and the "found no GPU processes" line saying why. The extension is that
`not_followed_notice` fires for plain auto-selection as well as `--follow-new`. The production logic is
about 60% of the production diff and is mostly right: `relevant_denied`, the fourth parameter of
`ps_exit_code`, `remedy_clause`, `failure_detail`, `denied_pid_notices` and the `missing_pid_notices`
filter are small and sit in the idiom of the maintainer's sibling helpers.

The weight is in the tests and in test machinery. In the three `src/bin/hmn` files the item adds about
266 lines of code and docs and about 369 lines of tests (30 new tests). `tests/cli_ps.rs` adds 283 lines
to deliver what R01 states in two sentences: accept exit 2 only with a skip line that carries
`process list unreadable:`, or on a VM the `NoGpuSource` text, "under its own label". Every one of the A8
additions (`Branch`, `is_denial_line`, the tie test, `catch_unwind`) tests the test helper, not
`hmn`. They came from the A8 rule "every new test fails when its fix is reverted", applied to mutants of
the test harness. The maintainer's own `tests/cli_ps.rs` has one test and no helper tests. His review of
PR B asked for stricter behaviour, not for tests of the test.

Two `watch.rs` seams (`sample_failed_line`, `no_processes_line`) are not worth their cost.
`not_followed_notice` is borderline. `unreadable_clause` duplicates `ps::remedy_clause`.

A smaller diff that keeps all the behaviour: about 920 added lines (src/bin plus `cli_ps.rs`) fall to
about 370.

## 1. Asked or added: behaviour against the maintainer and R01

| Behaviour | Source | Delivered | Notes |
|---|---|---|---|
| `N unreadable — re-run outside the sandbox` | maintainer, R01 | yes | `remedy_clause`, one remedy for both counts, as R01 says |
| `--pid` limits denied PIDs the way `judge` limits rows | R01 | yes | `relevant_denied` |
| `--exit-status` 2 when nothing listed and a relevant PID denied | R01 | yes | the 4th parameter |
| skip line and `--device` line carry the remedy | R01 ("the skipped-device line ... use it") | yes | `failure_detail`, used at 3 sites |
| `watch`: one notice per denied explicit PID; denied kept out of `missing_pid_notices` | R01 | yes | `denied_pid_notices`, extra filter |
| `watch`: "found no GPU processes" says why | R01 | yes | |
| `watch`: `--follow-new` gets a count | R01 | yes, widened | the notice also fires for plain auto-select (`!explicit && watching > 0`); R01 says follow-new only |
| `watch`: per-interval error path does not repeat the remedy | R01 | yes, trivially | the remedy is not in `Display`, so the original `({e})` already satisfies it |
| JSON unchanged, notices on stderr | R01 | yes | |
| CI: exit 2 only with the denial line, else `NoGpuSource` text, own label | maintainer, R01 | yes, but over-built | see section 2 |

**Missing or weaker than agreed: nothing found.** One note: PR B's CI record
(`__reports__/v0214_part1/ci_macos_cli_ps.md`) shows `branch=expected` on all four `macos-latest` jobs.
The new branches are therefore unlikely to fire on CI. The evidence he asked for is the label line plus
a green run, and a three-line `if` chain gives exactly that. The denial-line text itself is only
exercised by the `#[ignore]`d sandbox test, which CI does not run. So the non-ignored unit tests of
`is_denial_line`, `branch` and the others are the only part of this machinery that CI executes, and they
execute string handling in the test file, not `hmn`.

**Smallest diff in the maintainer's style.**

- `ps.rs`: keep the production changes as they are. Cut the tests to one table row-set for
  `ps_exit_code`, two `relevant_denied` tests, three summary tests and one `failure_detail` test.
- `watch.rs`: keep `denied_pid_notices` plus the `missing_pid_notices` filter, each with its test.
  Inline the rest in `run_watch`, as the maintainer left the "found no GPU processes" `eprintln!`s
  inline. Reuse `ps::remedy_clause` for the count clause.
- `cli_ps.rs`: extend `accept`'s existing `if` chain.
- CHANGELOG: four bullets become two (`ps`, `watch`).

## 2. The A8 test machinery in `tests/cli_ps.rs`

Evidence for A8 is in `13-amendment_a8_pwu_reviews_v0.md`: a test reviewer ran 53 mutants. Surviving
mutants of the harness (`accept`'s third branch accepting any exit, labels collapsed, a doubled
remedy on the skip line) justified new tests. The amendment itself records "Not adopted: a
`#[should_panic]` test of `accept` (the pure `branch` covers it)". The `catch_unwind` test is that test
under another name.

| What | Lines | Asked? | Improves? | Idiom | Verdict |
|---|---|---|---|---|---|
| `explained_by_denial`, `skipped_for_no_gpu_source` (fns) | ~22 | R01 asked for the two acceptance cases and the "same line" rule | The semantics are right: the needle must sit on a skip line. | Two near-identical fns; one `skipped_device_line(stderr, needle)` does both. | SIMPLIFY |
| tests of those two fns | ~45 | A8 review | They test the helpers, which CI uses only as an `assert`. | Not his idiom; his `cli_ps` has no helper tests. | DROP |
| `Branch` enum + `label()` | ~25 | A8 | Replaces three string literals with an enum. His `RemedyPurpose` nit was about a magic string that changes behaviour; here the strings only feed a log line and an `assert_ne`. | Ceremony: the enum exists so a pure `branch()` can be tested. | DROP (keep string labels) |
| `branch()` + its table test (incl. the "both texts" tie row and the labels-distinct check) | ~60 | A8 | The "line carries both texts" case cannot arise: `ProcessListDenied`'s `Display` never contains the `NoGpuSource` text. Labels-distinct re-asserts string literals. | Ceremony. | DROP |
| `is_denial_line` (whole-line parser) | ~17 | A8 (doubled remedy mutant) | The doubled remedy is also pinned exactly by `ps.rs` and `format.rs` unit tests, which run on every CI leg. | A 4-stage `strip_*` chain where he writes `contains`/`starts_with`. | DROP; assert `contains(DENIAL_TEXT)` and `contains("re-run outside the sandbox")` on the skip line (about 3 lines) |
| test of `is_denial_line` | ~28 | A8 | Tests a helper used only in an `#[ignore]`d test. | Ceremony. | DROP |
| `accept_fails_on_a_bare_exit_2` (`catch_unwind`) | ~8 | A8 | Tests `assert_ne!`. Contradicts the amendment's own "not adopted". | Ceremony; also prints a panic message into the test output. | DROP |
| `the_texts_this_file_matches_are_the_librarys_display_prefixes` ("tie" test) | ~14 | A8 | Real but cheap: a reword of `Display` would turn `skipped-device` into `rejected` and fail the macOS CI job anyway. | Defensible; no precedent in his tests. | DROP (or keep, 14 lines; marginal) |
| widened ignored test (`--device` line, denial on the skip line) | ~20 | R01 spirit | Real evidence of the user-facing line under a real sandbox. | Fine. | KEEP, with the `contains` asserts |
| consts `DENIAL_TEXT`, `NO_GPU_SOURCE_TEXT` | ~6 | R01 | Fine. | Fine. | KEEP |

Replacement, in the shape that already exists:

```rust
fn skipped_device_line(stderr: &str, text: &str) -> bool { /* existing body + `&& l.contains(text)` */ }
...
} else if code == Some(2) && skipped_device_line(stderr, DENIAL_TEXT) { "skipped-device" }
else if code == Some(2) && skipped_device_line(stderr, NO_GPU_SOURCE_TEXT) { "skipped-device-nogpu" }
else { "rejected" }
```

That is about +30 lines in all, against +283 now. It produces the same label line in CI's log and the
same rejection of a bare exit 2.

## 3. The two extra `watch.rs` seams, and the rest

Context: the maintainer does factor attach notices into pure helpers (`missing_pid_notices`,
`spilling_at_attach_notice`, `format_followed_set_change`, each with `impl Fn` injection where it
needs it). He left the "found no GPU processes" `eprintln!`s inline in `run_watch`.

| What | Lines (code+doc / tests) | Asked? | Improves? | Idiom | Verdict |
|---|---|---|---|---|---|
| `denied_pid_notices` | 18 / ~20 | R01 | Sibling of `missing_pid_notices`; names the exact text. | His idiom. | KEEP. Merge its two tests into one. |
| `missing_pid_notices` `denied` parameter + test | 3 / 8 | R01 | Prevents a double notice for one PID. | Fine. | KEEP |
| `unreadable_clause` | 12 / ~16 | not asked | Duplicates `ps::remedy_clause(0, n, outside)` (same text, same `remedy_text` call); `watch.rs` already imports from `ps`. | Duplicate. | DROP; make `remedy_clause` `pub(crate)`. Saves about 28. |
| `no_processes_line` (the extra seam) | 28 / ~28 | R01 asked for the *message*, not a seam | The original two `eprintln!`s took two `{because}` insertions; a one-line `let because = ...` suffices. One of the two tests only pins the old, unchanged wording. `run_watch` wiring is untested either way. | Not what he did with this message. | DROP (inline); about −50 net |
| `not_followed_notice` (the other extra seam) | 20 / ~23 | R01 (`--follow-new` gets a count); auto-select added | The `bool` logic (`!explicit && watching > 0`) is trivial and its wiring (passing `explicit` as `false`) is the mutant A8 admits nothing catches; the seam's unit test cannot catch it. Mild scope growth: auto-select also prints it. | Ceremony for a 3-condition `if`. | SIMPLIFY: inline as `if explicit.is_empty() && !watched.is_empty() { if let Some(c) = &unreadable { eprintln!(...) } }`, about 6 lines, no tests; about −35. Keep the auto-select wording or restrict to `--follow-new` per R01. |
| `sample_failed_line` | 6 / ~22 | R01 line "per-interval error path does not repeat the remedy" | The original inline `eprintln!` already prints `{e}` and `Display` carries no remedy. The helper changes nothing; it exists to pin a non-change. | Ceremony. | DROP; revert to the original inline call; about −28 |
| `run_watch` wiring | ~25 | R01 | Needed. | Fine. | KEEP |

Verdict on the two seams: `no_processes_line` and `sample_failed_line` were not worth adding.
`not_followed_notice` is borderline and does not earn its tests. `watch` is the surface where the sandbox
failure arrives mostly as an attach error (`failure_detail`, exit 2) or as a denied explicit PID; the
"nothing selected but some unreadable" case is narrow, so the extra machinery sits on the least likely
path.

## 4. `ps.rs` and `format.rs` table

| What | Where | Lines (code / tests) | Asked? | Improves? | Idiom | Verdict | Δ |
|---|---|---|---|---|---|---|---|
| `SummaryNotes::unreadable` | `ps.rs:181` | 4 / 0 | R01 | yes | yes | KEEP | 0 |
| `PsFilters::relevant_denied` | `ps.rs:293` | 10 / 36 (5 tests) | R01 | yes; one `contains` is the whole logic | tests over-count: 5 tests for 5 lines | SIMPLIFY to 2 tests | −16 |
| `ps_exit_code` 4th parameter | `ps.rs:~400` | 4 / 17-row table + 6 more tests | R01 | yes | the table already covers all 16 cells; the 6 extra tests repeat table rows | KEEP the table, DROP the 6 tests | −45 |
| `remedy_clause` | `ps.rs:~590` | 22 / 4 unit tests + 4 summary tests | R01 ("said once") | yes; reusable by `watch` | summary tests already pin it; unit tests duplicate | KEEP fn, DROP the 4 unit tests and the ordering test | −45 |
| `run_ps` wiring (`notes.unreadable +=`, `failure_detail` in 2 arms) | `ps.rs:~498` | 8 / 0 | R01 | yes | `failure_detail` computed in both arms; fine | KEEP | 0 |
| `ps` skip-line test of `failure_detail` | `ps.rs` tests | 0 / 17 | R01 | re-pins the `format.rs` test through `device_query_failure_line` | duplicate | DROP | −17 |
| `format::failure_detail` | `format.rs:~207` | 11 / 2 tests (35) | R01 ("skipped-device line ... use it") | yes: one wording at 3 sites | fine | KEEP fn; one test, no "unreachable build" case, no 3-error loop | −20 |
| help text (`main.rs`, +23/−12) | | | R01, issue | yes; p_comm cut was asked in R01 | yes | KEEP | 0 |
| CHANGELOG, item 6 bullets | | ~35 | roadmap item 11 | 4 bullets overlap each other and the item 4 bullet (`gpu_processes ... answer ProcessListDenied` repeats the `hmn ps` bullet) | longer than his entries | SIMPLIFY to 2 bullets (`ps`, `watch`) | −15 |

## Changes I would make first (rough size after)

1. `tests/cli_ps.rs`: replace `explained_by_denial`, `skipped_for_no_gpu_source`, `Branch`, `branch()`,
   `is_denial_line`, the tie test, the `catch_unwind` test and the four helper tests with a
   `skipped_device_line(stderr, text)` and two extra `else if` arms; assert the remedy by `contains` in
   the ignored test. Delivers the same label lines in CI. +283 becomes about +35.
2. `watch.rs`: delete `sample_failed_line`, `unreadable_clause` and `no_processes_line`; inline
   `not_followed_notice`; use `ps::remedy_clause` for the count. Keep `denied_pid_notices` and the
   `missing_pid_notices` filter. About 270 lines (code and tests) becomes about 110.
3. `ps.rs` tests: delete the 6 `ps_exit_code_*` tests that repeat the table, the `remedy_clause` unit
   tests, the duplicate `failure_detail` test, and 2 of 4 `relevant_denied` tests. About −125 lines.
4. `format.rs` tests: one `failure_detail` test (denial with remedy, one other error unchanged). −20.
5. CHANGELOG: two item 6 bullets in place of four, with the `ProcessListDenied` bullet folded into the
   `gpu_process_listing` entry. −15.

After these: roughly 920 added lines in `src/bin/hmn` and `tests/cli_ps.rs` become about 370, with every
user-visible line, exit code and CI label unchanged. Gaps left unprotected (the `explicit = false`
wiring mutant; `&[]` passed to `missing_pid_notices`) are already listed as residuals in A8 and PR B
accepted "pinned by grep rather than a run" residuals of this kind. The sandboxed
`--ignored` test and the field check are the right evidence for them.
