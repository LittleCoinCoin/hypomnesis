# Gap Analysis: v0214-sandbox — kinfo_enumeration revised after its three reviews (A6)

## Problem Statement
`kinfo_enumeration` was implemented on `task/kinfo_enumeration` (578bf07 red, 761312a, e76524b, 201a1db, on b716087). Every Success Gate and Consistency Check passed, re-run by the spec verifier on each commit. Three reviewers then attacked it, each in its own worktree. The spec verifier (Sonnet) gave PASS WITH NOTES, with 2 should-fix and 7 notes. The independent FFI reviewer, which had no part in the code or its spec, gave PASS WITH NOTES: 3 should-fix, 6 notes, no memory-safety problem and no UB, with errno matching XNU under none, P, S, S0, Q, D, L and C. The test reviewer gave PASS WITH NOTES: 6 should-fix, 7 notes, about 90 mutants. All 34 new tests fail when their fix is reverted (578bf07: 33 failed, 1 passed by construction, as specified). The gap is that several plausible wrong implementations of the new FFI layer pass every test.

## Evidence
The mutations that survive all 129 lib tests, natively and under Rosetta (reports in `11-a6_reviews/`):

| Wrong implementation | Caught by, at 201a1db |
|---|---|
| `namelen` 2, or `KERN_PROC_PID` in the `KERN_PROC_ALL` MIB | the hand-run harness gate `S --job` only |
| retry unbounded, or absent; the partial `ENOMEM` fill parsed; the fill's `rc` ignored; `len` not clamped | nothing: no seam injects `sysctl` results |
| `pidpath_failure` with EPERM and ESRCH swapped | harness profile D only |
| `kinfo::EPERM = 2` | nothing: the tests use the same symbol on the input side |
| `kern_proc_pid_comm` skipping `classify_kern_proc_pid` | nothing; runtime effect nil (a refused call's zeroed buffer reads as an empty comm, `None`) |
| `legacy_entries` dropping the rows it keeps | nothing: the test asserts `is_some()` only |
| `compare_names.py` on two all-`null` captures | passes (`compared 2 mismatch 0`); `""` accepted as a prefix |
| `count_denied.py` on an empty listing | passes (`denied=0 read=0 gone=0`) |

Docs that over-promise: `list_compute_processes`' rustdoc and the `gpu_processes` Metal-arm comment name only two of the six `None` causes. `denied_pids` does not say that PIDs ≤ 0 are skipped. The `proc_listpids` FFI doc's length formula differs from XNU's. The `None` sentence ("only when both `proc_pidpath` and `KERN_PROC_PID` are refused") omits a gone process, an empty `p_comm` and a non-refusal `proc_pidpath` failure. The Step 4 record names e76524b, which the squash re-creates.

## Root Cause
The leaf specified the `KERN_PROC_ALL` call as FFI with no pure seam, unlike `KERN_PROC_PID`, which PR B split into `kern_proc_pid_raw` and the pure `classify_kern_proc_pid`, so the retry and failure paths had no test that could reach them. The test list named `footprint_from_errno` but not its sibling `pidpath_failure`. The harness scripts were specified by their happy path.

## Impact Assessment
- Scope: `kinfo_enumeration` only. The fixes fold into the step that owns each file, as in A3: Step 2 (code, tests, rustdoc), Step 3 (the `None` sentence, the FAQ wrap), Step 4 (harness scripts and README, the record). The red Step 1 commit is squashed into Step 2 in the same rebase, so the branch ends with three commits, each green alone.
- New in the spec: `KinfoAttempt`, `classify_kinfo_all`, `KINFO_ALL_ATTEMPTS` in `kinfo.rs`; `fill_kinfo_all_with` and `comm_from_lookup` in `metal.rs`; 9 tests behind a new header gate. kinfo_enumeration adds 43 lib tests in all, and part2_close's Rosetta arithmetic becomes 95 + 43 + 9 − 2 = 145.
- Contracts with later leaves: unchanged. `list_processes`, `MetalProcessList`, `legacy_entries` and the harness interface (`sandbox.sh PROFILE --job`, `count_denied.py`'s output line, `compare_names.py`'s exit codes) keep their names and shapes.
- Residuals, stated in the record and carried to the PR body: `KinfoRead::Refused` and `Failed` are indistinguishable downstream (both `None`), by design; an absurd kernel probe size is not handled beyond `vec!`'s own allocation failure; libproc returning 0 without errno can at worst produce a `p_comm` name, never a denial. Wiring mutations reachable only on hardware stay pinned by the harness gates and the grep pin, as PR B's were.
- Not adopted: a separate commit for the review round. The branch is unpushed, so each fix goes where its file's step owns it.

## Proposed Solution
The kinfo_enumeration implementer applies Step 2 item 8, Step 3 item 6 and Step 4 item 4, using fixup commits and a non-interactive autosquash that also folds 578bf07 into Step 2. It re-runs every gate of the leaf and the full gate set on each of the three commits. The spec verifier then re-checks the delta, and the test reviewer re-runs the surviving mutants.

## Recommendations
1. Adopted by the coordinator under the brief's review requirement ("every new test must fail when its fix is reverted"); recorded for `__reports__/v0214_part2/01-reviews_v0.md`.
2. Use the same seam pattern (pure classifier beside the FFI call) for any further FFI in PR C.
