# Review of the tests added by `ps_watch_unreadable` (task/ps_watch_unreadable @ 9e0bb9b, on e7d6c2d)

**Verdict: PASS WITH NOTES** (0 blocker, 3 should-fix, 9 note)

Every new or widened unit test failed under at least one mutation of the code it guards (table below). The format and watch strings are pinned whole, so rewording one word in the clause, the joined clause, a watch notice or `sample failed` fails a test. The red commit 6d5a322 gives 14 `panicked at` and 0 `error[E`, as the leaf requires. The `#[ignore]`d sandbox test still fails rather than skips when it cannot sandbox. The gaps are at the edges the unit tests do not reach:

- The widened ignored test checks the skip line and the `--device` line with `contains` or `ends_with`. A remedy said twice, or a remedy with a suffix, passes it. The remedy said twice also passes the leaf's `[run]` gate.
- `accept`'s branch logic has no test. Every mutation of its third branch, and the removal of its rejection assert, passes the whole suite.
- `run_watch` wiring has one more survivor besides the one the implementer named: `not_followed_notice(.., explicit = false, ..)`. It passes every cargo test and the leaf's gate 22 as written.

Method: a detached worktree at 9e0bb9b (`scratchpad/v_pwu_tests`) for mutations, and a second one (`scratchpad/v_pwu_tests_lint`) for the lint swap, the release-build wiring gates and the red commit. Both are removed at the end. Each mutation was one replace-once edit, then `cargo test --locked --all-features --bin hmn --test cli_ps --no-fail-fast -- --include-ignored` (unsandboxed parent, so the ignored sandbox test ran), then a restore of the file. `git status --short` was empty after each batch. The scripts are `scratchpad/vpt_mut.py`, `vpt_mut2.py` and `vpt_wire.py`, and their outputs are `vpt_mut.out` and `vpt_wire.out`. Baseline: `--bin hmn` 279 passed, `cli_ps` 5 passed (ignored test included).

## Findings, most severe first

### 1. should-fix: the widened ignored test matches the denial lines by `contains` or `ends_with`, so a doubled or reworded remedy passes it
- `tests/cli_ps.rs:212-217` (skip line: `contains("process list unreadable: ") && contains(" — re-run outside the sandbox")`) and `:238-244` (`--device` line: `starts_with(..) && ends_with(" — re-run outside the sandbox")`).
- Evidence. Each mutant below passes all 279 + 5 tests:
  - W4: `run_ps` builds the skip-line detail as `format!("{} — {}", failure_detail(..), remedy_text(.., Names))`.
  - W5: the same change on the `--device` line.
  - W6: the skip-line detail is `format!("{e} — {}", remedy_text(.., Identify))`, which gives `… — re-run outside the sandbox to identify (skipped)`.

  The unit test `failure_detail_puts_the_remedy_on_the_skip_line_and_the_device_line` pins the composed string, but `run_ps` builds the line itself, so no unit test sees what `run_ps` does. The leaf's gate 18 regex `.* — re-run outside the sandbox (skipped)$` also accepts W4 and W5. Measured on release builds with `vpt_wire.py`: W4 and W5 print `P 2 1 / P device 2 1 / S0 2 1 / S0 device 2 1`, the same as baseline. W4's line reads `… could be read — re-run outside the sandbox — re-run outside the sandbox (skipped)`. Only W6 fails gate 18. So "the remedy said once per line", which the leaf states as a rule, is not pinned on the skip line or the `--device` line.
- Fix: pin each line whole except the count. Prototype (in `scratchpad/vpt_proto.diff`):
  ```rust
  l.strip_prefix("hmn: ps failed to query device 0: process list unreadable: ")
      .and_then(|r| r.strip_suffix(" refused, none other than the caller's could be read — re-run outside the sandbox (skipped)"))
      .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
  ```
  Use the same check for the `--device` line, without ` (skipped)`. The prototype passes unmutated and catches W4, W5 and W6. Gate 18's `.*` could become `[0-9]* refused, none other than the caller's could be read`.

### 2. should-fix: `accept`'s branches have no test; its third branch and its rejection assert can be mutated freely
- `tests/cli_ps.rs:138-153`.
- Evidence. Each mutant passes the suite:
  - C1: drop `code == Some(2) &&` from the `skipped-device-nogpu` branch, so a `NoGpuSource` skip line excuses any exit code.
  - C2: replace the `assert_ne!(branch, "rejected", ..)` with a no-op, so a `rejected` case passes silently.
  - C3: the third branch is labelled `skipped-device`. The two labels collapse, which the leaf's gate 20 says the change must catch.
  - C4: the third branch is PR B's `skipped_device_line(stderr)`, which accepts exit 2 with any skip line.

  On this host every `accept` call takes `expected`, and under profile P it takes `skipped-device` (gate 21: `exit 0`, `2`, `0`). The third branch therefore never runs anywhere but a macos-latest VM, and no unit test calls it. The only pin on it is the static gate `grep -c 'skipped-device-nogpu' ≥ 1`. To the brief's question, the unmutated `accept` does reject exit 2 with no skip line, and it does reject loudly. Both properties hold only because nobody has changed the code. No test enforces them.
- Fix: move the decision into `fn branch(code, stderr, expected) -> &'static str` and keep `accept` as label, log and assert. Add a table test: `(Some(1), "", 1)` gives `expected`; the denial line with `Some(2)` gives `skipped-device`; the `NoGpuSource` line with `Some(2)` gives `skipped-device-nogpu`; and the `NoGpuSource` line with `Some(1)` and expected 0, a bare `Some(2)`, and an out-of-range skip line give `rejected`. Add a `#[should_panic(expected = "expected 1 or 2 with a skip line")]` test of `accept("bare", Some(2), "", 1)`. In the prototype, C1, C3 and C4 fail the table test and C2 fails the `should_panic` test (my script's regex missed the `- should panic … FAILED` line; the run shows `6 passed; 1 failed`). Clippy `--all-targets --all-features -D warnings` passes on stable and 1.88, and `cargo fmt` needs a pass.

### 3. should-fix: a second `run_watch` wiring survivor: `not_followed_notice` told the watch is not explicit
- `src/bin/hmn/watch.rs:1302-1307` (`!selection.explicit.is_empty()`).
- Evidence. V2 (`false` in its place) passes every cargo test. On a release build, the leaf's gate 22 command prints `exit 0`, `1`, `0`, `3`, identical to baseline. The stderr gains `hmn watch: device 0: 1060 unreadable — re-run outside the sandbox; they are not followed` after the `pid=1 is unreadable here` notice, which my added count `notfollowed=1` shows (baseline `0`). An explicit watch then says the count twice and claims PIDs it was never asked to follow are "not followed". Gate 23 does not see it either, since its runs are all auto-select.
- The implementer's survivor V1 (`missing_pid_notices(.., &[], ..)`) reproduces: gate 22 prints the baseline values, because `process_exists(1)` is `Some(true)`.
- Fix (either):
  - add `notfollowed=$(grep -c "they are not followed" target/w.err)`, expected `0`, to gate 22;
  - better, move the attach-notice sequence (spilling, denied, missing, unmatchable, count line) into a pure `fn attach_notices(..) -> Vec<String>` that `run_watch` prints. One unit test then pins the leaf's stated order and catches V1 and V2, and V6 as well.

### 4. note: the boundary at 1 is not pinned by unit tests in two places
- `remedy_clause` (`src/bin/hmn/ps.rs:601`): P4 (`unreadable > 1`) survives the suite, because every ps test uses 907 or 0.
- `not_followed_notice` (`watch.rs:1072`): U1 (`watching > 1`) survives, because the tests use 2 and 0.
- Both cases are real. Gate 16's WindowServer run gives `1 unreadable`, and under S, auto-select watches exactly one row (hmn's own). So gates 16 and 23 catch them, but only on a machine with the harness, never in CI.
- Fix: add `remedy_clause(0, 1, true) == Some("1 unreadable — re-run outside the sandbox")` and `not_followed_notice(0, clause, false, 1).is_some()`. Each one caught its mutant in the prototype.

### 5. note: the order of the remedy clause and `K unnamed not matched` is unpinned
- `src/bin/hmn/ps.rs:748-751`. P7 (remedy clause pushed after the unnamed count) survives.
- The leaf says the clause goes "before `K unnamed not matched`". The gap is PR B's, since protected plus unnamed was not pinned either. It now applies to `--filter` under a sandbox, where both counts appear.
- Fix: `SummaryNotes { unreadable: 5, unnamed: 2, .. }` with `--filter canvas` gives `0 GPU processes found matching filter="canvas" (5 unreadable — re-run outside the sandbox; 2 unnamed not matched).` This caught P7 in the prototype.

### 6. note: `relevant_denied` with duplicates. `--pid` duplicates are safe, denied duplicates count twice, and nothing pins either
- `src/bin/hmn/ps.rs:292-298`. Measured with a throwaway test:
  - `filters(&[2, 2])` gives 1 over `[1, 2, 3]`. A raw `PsFilters { pids: vec![2, 2] }` also gives 1, because the count iterates `denied`. `PsFilters::new` dedups `--pid` as well.
  - With no `--pid`, `[2, 2, 3]` gives 3. With `--pid 2`, it gives 2.
- R1 (count over `--pid` instead of over `denied`) and R3 (dedup `denied`) both survive. They differ from the code only when `denied` holds a duplicate.
- `GpuProcessListing::denied_pids` is documented "in enumeration order", and its rustdoc does not promise uniqueness.
- Fix: say in the rustdoc that `denied_pids` holds each PID once, which is what a count needs. Or pin the current behaviour with a `[2, 2, 3]` case so a later change is deliberate.

### 7. note: the `ps_exit_code` tests cover 13 of the 16 cells
- The domain is {exit_status} × {rows_empty} × {failed 0, >0} × {relevant_denied 0, >0}. The cells not covered are `(false, false, ·, ·)` with a non-zero count: (F,F,1,0), (F,F,0,1) and (F,F,1,1).
- Every realistic arm-order mutant fails a test:
  - E5, an unguarded denial arm first, fails `ps_exit_code_without_exit_status_is_zero` and `ps_exit_code_listed_rows_beat_denials_and_failures_is_zero`.
  - E6, an unguarded failed arm first, fails those two and the table test.
  - E2 (`> 1`) and E3 (a denial beats rows) fail a test each.
- E1 was an equivalent mutant, because its first arm was guarded. Only the contrived E4, which differs on (F,F,0,1) alone, survives.
- Fix: one row `((false, false, 1, 1), 0)` in the table makes it total.

### 8. note: `skipped_for_no_gpu_source`'s `!l.contains(DENIAL_TEXT)` is dead
- `tests/cli_ps.rs:80`. C5 (the clause dropped) survives. In `accept`, `explained_by_denial` is tried first, so a line that holds both texts never reaches the third branch. The unit test has no line with both texts.
- Fix: drop the clause, or add a case with both texts to the unit test.

### 9. note: wiring that no cargo test reaches, caught only by a `[run]` gate or by no gate at all
- Caught by a gate: W1 (unfiltered count) by gate 16; W2 (`ps_exit_code(.., 0)`) by the static grep, gate 24; V3 (the attach error printed without `failure_detail`) and V7 (`no_processes_line(.., None)`) by gate 23; V6 (`denied_pid_notices` given `&[]`) by gate 22. This matches the implementer's `wiring.log`.
- Caught by no assertion:
  - W8 (`run_ps` calls `format_ps_summary_with(.., false)`). Reasoned, not run: the summary would read `N unreadable — re-run elevated for names` under S, and gates 15 and 16 count `unreadable` without asserting the text after it. The call into the wrapper is remedy_macos's and is unpinned there too.
  - V4 (the interval line built with `failure_detail`). This path is not reachable: a sandbox does not change after a successful attach.
- Equivalent mutants: a literal `true` at any `REMEDY_OUTSIDE_SANDBOX` call site (V5, and the `run_ps` sites) is equivalent on macOS and cannot be reached elsewhere, as finding 10 shows. W3 (the `--device` line printed without `failure_detail`), which the implementer's table lists as a survivor, is now caught by the widened ignored test's `--device` run. That run was added after the implementer's table.
- Fix (optional): in gate 16, add `grep -c "(1 unreadable — re-run outside the sandbox)\.$"` = 1.

### 10. note: the off-macOS invariant holds structurally, and its unit tests are not vacuous
- Every new clause needs a non-empty `denied_pids` or a non-zero count. The lib pins `denied_pids` empty for the NVML, PDH and nvidia-smi arms (`listing_without_denials_has_no_denied_pids`, which runs on every OS).
- `failure_detail` appends the remedy only for `ProcessListDenied`, which is never returned off macOS. F2 (the remedy on every error) fails `failure_detail_leaves_other_errors_alone`, which loops over both flag values.
- A literal `true` in `format_ps_summary_with` (P6) fails the six pinned Windows and Linux strings and `format_ps_summary_unreadable_zero_changes_nothing`.
- Every new test passes the flag as a literal, so none needs a `#[cfg]` twin, and none passes vacuously on a runner with no GPU.
- One weak half: in `failure_detail_leaves_other_errors_alone`, the `NoGpuSource` half compares against `NoGpuSource.to_string()`, which is tautological. The `DeviceIndexOutOfRange` half is literal and carries the test.

### 11. note: a stale doc comment in `tests/cli_ps.rs:155-161`
- It still says a GPU source fails under "a sandbox that refuses `proc_listpids`" and that "`accept` takes that branch only with the line".
- After kinfo_enumeration, such a sandbox lists through `KERN_PROC_ALL`. A process-info denial now gives the denial skip line (`skipped-device`), and a VM gives the `NoGpuSource` line (`skipped-device-nogpu`), so there are two branches.
- `accept`'s first sentence, "only together with its skip line", also reads as "any skip line", while an out-of-range skip line is now rejected.

### 12. note: small idiom points
- `watch.rs:2440` writes `not_followed_notice(..).unwrap()`. It is legal under the module's `#[allow(clippy::unwrap_used)]`, which fires (finding below). `assert_eq!(not_followed_notice(..).as_deref(), Some("…"))` would need no unwrap.
- The `matches("re-run").count() == 1` and `!contains("re-run")` assertions that follow a whole-string `assert_eq!` are redundant. They are harmless.
- The ignored test spells `"process list unreadable: "` instead of `DENIAL_TEXT`.

## Mutation table

| Mutation | Test that caught it |
|:--|:--|
| F1 `failure_detail`: ` — ` → ` – ` | `failure_detail_appends_the_remedy_to_a_denial`, `failure_detail_puts_the_remedy_on_the_skip_line_and_the_device_line`, `ps_exits_2_…` (ignored) |
| F2 `failure_detail`: remedy on every error | `failure_detail_leaves_other_errors_alone` |
| F3 `failure_detail`: `Names` → `Identify` | the two `failure_detail_*` tests above, `ps_exits_2_…` |
| F4 `failure_detail`: flag ignored (`true`) | `failure_detail_appends_the_remedy_to_a_denial` |
| P1 `remedy_clause`: `, ` → ` and ` | `format_ps_summary_unreadable_and_protected_say_the_remedy_once`, `remedy_clause_unreadable_and_protected_share_one_remedy` |
| P2 `unreadable` → `unreadable processes` | 4 tests (`format_ps_summary_unreadable_*` ×2, `remedy_clause_*` ×2) |
| P3 protected before unreadable | `…say_the_remedy_once`, `…share_one_remedy` |
| P4 `unreadable > 0` → `> 1` | **survived** (gate 16 only; finding 4) |
| P5 ` — ` → `: ` in the clause | 14 tests, the six Windows and Linux pins included |
| P6 summary passes literal `true` | the six pinned Windows and Linux strings, `format_ps_summary_unreadable_zero_changes_nothing` |
| P7 remedy clause after the unnamed count | **survived** (finding 5) |
| P8 unreadable folded into the protected count | `format_ps_summary_unreadable_counts_with_the_macos_remedy`, `…say_the_remedy_once` |
| R1 `relevant_denied` iterates `--pid` | **survived**, equivalent unless `denied` has duplicates (finding 6) |
| R2 count capped at `pids.len()` | `relevant_denied_for_a_pid_nobody_denied_is_zero` |
| R3 dedup `denied` | **survived** (finding 6) |
| E1 guarded denial arm first | equivalent mutant |
| E2 `relevant_denied > 1` | `ps_exit_code_empty_and_a_relevant_pid_denied_is_two` |
| E3 a denial beats listed rows | `ps_exit_code_listed_rows_beat_denials_and_failures_is_zero` |
| E4 differs on (F,F,·,>0) only | **survived** (contrived; finding 7) |
| E5 unguarded denial arm first | `…without_exit_status_is_zero`, `…listed_rows_beat_…` |
| E6 unguarded failed arm first | the same two, and the table test |
| W1 `run_ps` counts every denied PID | **survived** cargo; gate 16 |
| W2 `ps_exit_code(.., 0)` | **survived** cargo; static gate 24 |
| W3 `--device` line without `failure_detail` | `ps_exits_2_…` (ignored) |
| W4 skip line says the remedy twice | **survived**, gate 18 included (finding 1) |
| W5 `--device` line says the remedy twice | **survived**, gate 18 included (finding 1) |
| W6 skip line remedy `… to identify` | **survived** cargo; gate 18 |
| W7 skip line with `!REMEDY_OUTSIDE_SANDBOX` | `ps_exits_2_…` (ignored) |
| W8 summary via `format_ps_summary_with(.., false)` | **survived** cargo; no gate asserts it (finding 9) |
| U1 `not_followed_notice`: `watching > 1` | **survived** (gate 23 only; finding 4) |
| U2 `unreadable_clause`: `count > 1` | `unreadable_clause_is_none_for_zero` |
| U3 denied notice: ` — ` → `, ` | `denied_pid_notices_name_each_denied_explicit_pid_in_the_order_given` |
| U4 `no_processes_line`: `; ` → `, ` | `no_processes_line_says_why_inside_the_parentheses` |
| U5 `not followed` → `not watched` | `not_followed_notice_states_the_count_and_the_remedy_once` |
| U6 `skipping interval` → `skipped interval` | both `sample_failed_line_*` |
| U7 `missing_pid_notices`: denied filter dropped | `missing_pid_notices_skip_denied_pids` |
| U8 denied notices in `denied` order | `denied_pid_notices_name_each_…_in_the_order_given` |
| V1 `missing_pid_notices(.., &[], ..)` | **survived**, gate 22 included (implementer's survivor) |
| V2 `not_followed_notice(.., false, ..)` | **survived**, gate 22 included (finding 3) |
| V3 attach error printed with `{e}` | **survived** cargo; gate 23 |
| V4 interval line built with `failure_detail` | **survived**; path not reachable |
| V5 `unreadable_clause(.., true)` | equivalent on macOS |
| V6 `denied_pid_notices(.., &[], ..)` | **survived** cargo; gate 22 |
| V7 `no_processes_line(.., None)` | **survived** cargo; gate 23 |
| C1 nogpu branch without `code == Some(2)` | **survived** (finding 2) |
| C2 `accept` never asserts | **survived** (finding 2) |
| C3 nogpu labelled `skipped-device` | **survived** (finding 2) |
| C4 nogpu branch accepts any skip line | **survived** (finding 2) |
| C5 `skipped_for_no_gpu_source` without `!DENIAL_TEXT` | **survived**, a dead clause (finding 8) |
| C6 `explained_by_denial` ignores the exit code | `explained_by_denial_needs_exit_2_and_the_denial_on_one_skip_line` |
| C7 skip line and denial on any lines | the same |
| C8 denial text not required | the same |

With the prototype from findings 1, 2, 4 and 5 (`scratchpad/vpt_proto.diff`, since reverted), P4, P7, U1, W4, W5, W6, C1, C2, C3 and C4 each fail a test.

## Answers to the brief's specific attacks

- **Whole-string pins.**
  - Pinned whole: the `N unreadable — re-run outside the sandbox` clause, the joined `unreadable, protected` clause, the three summaries, every watch notice (denied, missing, count line, both `found no GPU processes` forms) and `sample failed`. A one-word reword of each fails a test (P2, P1, U3, U4, U5, U6).
  - The composed skip line and device line are pinned whole in `failure_detail_puts_…`. As `run_ps` prints them, they are checked only by `contains` and `ends_with` (finding 1).
  - Not pinned by any unit test: the `hmn: watch failed to query device D: …` attach line. It is built inline in `run_watch`, and gate 23 is its only check.
- **Off-macOS invariant.** No new path prints `unreadable` or the macOS remedy without a non-empty denied list (finding 10). The literal-`true` mutants are equivalent on macOS and cannot be reached elsewhere.
- **`cli_ps` acceptance.**
  - Unmutated, `accept` rejects exit 2 with no skip line, and it fails loudly on `rejected`. Nothing tests that (finding 2).
  - The ignored test still fails rather than skips. Run under profiles C, S and D through `sandbox.sh`, it exits 101 at the helper (`tests/cli_ps.rs:186`, "sandbox-exec cannot apply profile P here … sandbox_apply: Operation not permitted").
  - Under P it passes, because a nested P applies on this host. That is a real sandboxed run, not a skip.
  - Gate 21 under P printed `exit 0` with two `branch=skipped-device` lines. Run with `--include-ignored` under P, all 5 tests pass.
- **`ps_exit_code` table.** It covers 13 of 16 cells, and every realistic wrong arm order fails a test (finding 7).
- **`relevant_denied` with duplicates.** Duplicates in `--pid` count once. Duplicates in `denied` count twice. Neither case is pinned (finding 6).
- **Wiring survivors.** V1 is confirmed. V2 is a second one, and it also survives gate 22. W4 and W5 survive gate 18. W8 has no asserting gate. See findings 3 and 9.

## What I checked and found sound

- **Red commit 6d5a322.** The Step 1 command gives rc 101, `--bin hmn` 9 passed / 12 failed and `cli_ps` 0 passed / 2 failed, with 14 `panicked at` and 0 `error[E`, which meets the ≥ 14 and 0 the leaf requires.
- **Filter counts at the tip.**

  | Filter | Listed |
  |:--|:--|
  | `format_ps_summary_unreadable` | 3 |
  | `remedy_clause` | 4 |
  | `relevant_denied` | 4 |
  | `ps_exit_code` | 7 |
  | `failure_detail` | 3 |
  | `denied_pid_notices` | 2 |
  | `missing_pid_notices` | 2 |
  | `unreadable_clause` | 2 |
  | `sample_failed_line` | 2 |
  | `unresolved_growth_hint` | 2 |
  | `explained_by_denial_needs…` | 1 |

  Each matches the leaf. The extra `no_processes_line_*` (2) and `not_followed_notice_*` (2) tests are beyond the leaf and pin real strings.
- **No `#[allow]` was added.** The leaf's new `.unwrap()` relies on the existing `watch.rs` test-module allow. I swapped all four test-scope allows that cover the touched tests to `#[expect]`: `watch.rs:1415` and `format.rs:587` (`unwrap_used`), and `tests/cli_ps.rs:17` and `:176` (`expect_used`). `cargo clippy --locked --all-targets -- -D warnings` then gives rc 0 with 0 unfulfilled expectations under stable default, stable `--all-features`, stable `--all-features --target x86_64-unknown-linux-gnu` (where `cli_ps:176` is cfg'd out), `+1.88` default and `+1.88 --all-features`.
- **House idiom.**
  - The diff has no `cfg!`, `panic!`, `unreachable!`, `_ =>` arm over a crate enum or `as` cast.
  - `failure_detail` uses `matches!`.
  - `ps::tests` has no unwrap or expect.
  - `#[cfg(target_os = "macos")]` comes before `#[test]`, and the ignore reason starts with "requires".
  - The tests are small and literal: 907, 908, `[7, 3, 9]` against `[9, 100, 7]`, and `+12.3s` from `12.34`, which pins the rounding.
  - Every new emptiness check uses `assert!(x.is_empty(), "{x:?}")`.
- **Not tautological.** The expectations are literals and none restates a production constant. The one exception is the `NoGpuSource` half noted in finding 10. Each new test carries a positive control (`denied_pid_notices_skip_readable_pids`, `relevant_denied_for_a_pid_nobody_denied_is_zero`, `unreadable_clause_is_none_for_zero`, `not_followed_notice_is_for_auto_selection_…`).
- **Clean-up.** The mutation worktree was clean after every batch and after the prototype was reverted. Both worktrees are removed.
