# Gap Analysis: v0214-sandbox — gpu_process_listing revised after its reviews (A7)

## Problem Statement
`gpu_process_listing` was implemented on `task/gpu_process_listing` (38c151b red, da3d698, c456939, on 8519f1a). The spec verifier (Sonnet) re-ran every gate and found all passing; its verdict was PASS WITH NOTES, with 1 should-fix and 4 notes. The public API changes are additive only, and the seam into `decide_listing` is untouched. The test reviewer gave PASS WITH NOTES, with 4 should-fix and 6 notes from about 50 mutants. Every new test fails under some mutation of its fix, but four mutants survive every test that can run in CI.

## Evidence
Reports in `12-a7_reviews/`.

| Wrong implementation | Caught by, at c456939 |
|---|---|
| `listing_without_denials` returning a denied PID; the nvidia-smi arm returning `ProcessListDenied` | nothing on macOS or the Linux target. `gpu_process_listing_has_no_denied_pids_off_macos` passes vacuously on a runner with no GPU (`Err(NoGpuSource)`) |
| a Metal arm that adds a denied PID unsandboxed | nothing: the sandbox test's unsandboxed row does not check `denied=0` |
| `Display` with `; run it outside`, `; run as root`, `; Sandbox refused it`, or a count of `9080` | nothing: the test checks a word list |
| a truncating `as u32` count | nothing: `usize::MAX` truncates to `u32::MAX` |

Stale text: the `tests/smoke.rs` comment saying "on macOS only NoGpuSource is accepted", and the outcome list in `tests/macos_smoke.rs`. The `ProcessListDenied` variant doc says "every process but its own", which is narrower than the code (profile L). The leaf's sandbox gate also claims to catch "the caller listed as denied", which only `tally_reads_never_lists_the_caller_as_denied` catches.

## Root Cause
The leaf pinned the Linux and Windows invariant with an integration test that cannot run without a GPU, plus a call-site count. It pinned `Display` by forbidden words, not by value.

## Impact Assessment
- Scope: `gpu_process_listing` only. The fixes fold into Steps 1+2 (tests, `error.rs` doc), with the red commit squashed in the same rebase, so the branch ends with two commits.
- New: one lib test (`listing_without_denials_has_no_denied_pids`) and one static count. part2_close's Rosetta arithmetic becomes 95 + 43 + 10 − 2 = 146.
- Residuals for the PR body:
  - The Metal arm's wiring is caught only by the `#[ignore]`d `tests/macos_sandbox.rs`.
  - On stable, the `compile_fail` doctest passes for any compile error.
  - A generic `gpu_processes` parameter that still coerces is caught only by the integration tests' literal calls.
- Not adopted: the signature guard is not widened. Its mutation is a return-type change, which it catches.

## Proposed Solution
The implementer applies Step 2 item 8 and the new header gate as fixups, squashes 38c151b into Step 2, re-runs the leaf's gates and the shared gate set on both commits, and proves each new pin fails under its mutant. The spec verifier then re-checks the delta.

## Recommendations
1. Adopted by the coordinator under the brief's rule that every new test fail when its fix is reverted.
2. For ps_watch_unreadable: pin user-facing strings byte for byte, and pin an off-macOS invariant with a test that cannot pass vacuously.
