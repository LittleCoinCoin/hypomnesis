# Gap Analysis: v0214-sandbox — ps_watch_unreadable revised after its reviews (A8)

## Problem Statement
`ps_watch_unreadable` was implemented on `task/ps_watch_unreadable` (6d5a322 red, 6d97822, eee3100, 9e0bb9b, on e7d6c2d). The spec verifier (Sonnet) re-ran every gate and check on each commit, and all passed. It found the seam exact: in one S sandbox, hmn counted `1058 unreadable` and the probe `denied=1058`, in 4 of 4 runs. Its verdict was PASS WITH NOTES, with 0 should-fix and 7 notes. The test reviewer gave PASS WITH NOTES, with 3 should-fix and 9 notes from 53 mutants. All whole-string unit pins hold, but four mutants survive the cargo tests: two in `tests/cli_ps.rs` and two in `run_watch`'s wiring.

## Evidence
Reports in `13-a8_reviews/`.

| Wrong implementation | Caught by, at 9e0bb9b |
|---|---|
| the skip or `--device` line saying the remedy twice, or ending `… to identify` | nothing: the ignored test uses `contains`/`ends_with`, and gate 18's regex accepts a doubled remedy |
| `accept`'s third branch accepting any exit; its rejection assert removed; labels collapsed; PR B's "any skip line" rule restored | nothing: no test covers `accept`'s branches |
| `run_watch` passing `explicit = false` to `not_followed_notice` | nothing, gate 22 included |
| the summary count printed without the macOS remedy | nothing: gates 15 and 16 count `unreadable` only |
| `remedy_clause` with `> 1` instead of `> 0` | nothing below the CLI gates |

Text that over-promised: the `hmn ps --help` sentence "every process the caller's sandbox lets it inspect is listed", which is false for readable processes holding no GPU memory. Stale: the doc of `ps_exit_status_is_1_when_nothing_is_listed_and_opt_in`. The test's `DENIAL_TEXT` and `NO_GPU_SOURCE_TEXT` are hand copies of the library's `Display`.

## Root Cause
The leaf pinned the CLI test's acceptance by labels written to the log, not by a pure decision. Its CLI gates counted a token where the text after it carried the meaning.

## Impact Assessment
- Scope: `ps_watch_unreadable` only, folded into Step 2 (with the red commit squashed in) and Step 3 (the gate-22 wiring is Step 3's code). No lib test is added, so part2_close's 146 stands. `tests/cli_ps.rs` gains a `branch` table test and a `Display` tie test.
- Gate 20's awk pin moves from `fn accept` to `fn branch`.
- Residuals for the record and the PR body:
  - `run_watch` passing `&[]` to `missing_pid_notices` is caught by nothing.
  - The unfiltered accumulation in `run_ps` and an unwired `denied_pid_notices` are caught only by the CLI gates.
  - A duplicate denied PID would count twice, but the enumerations never list a PID twice.
- Not adopted: a `#[should_panic]` test of `accept` (the pure `branch` covers it), and moving the attach notices into one pure function (the gate-22 count covers the wiring).

## Proposed Solution
The implementer applies Step 2 item 7, the gate changes (15, 16, 20, 22) and the help wording as fixups, then squashes 6d5a322 into Step 2. That leaves three commits on e7d6c2d. It proves each new pin against its mutant and re-runs the gates. The spec verifier then re-checks the delta.

## Recommendations
1. Adopted by the coordinator under the brief's rule that every new test fails when its fix is reverted.
2. part2_close inherits notes N6 (the record's columns) and N7 (README item 9 and R01 line 315 still describe the profile-L residual that this leaf fixes).
