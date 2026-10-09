# Gap Analysis: v0214-sandbox — the maintainer's review of PR C (A10)

## Problem Statement
PR C (mi-for-the-rust-of-us/hypomnesis#8, head 1c97d88) received CHANGES_REQUESTED from PCfVW on 2026-10-08 (review 5453310396). He checked:
- the CI gate set on Windows and on Ubuntu WSL2 (stable and 1.88);
- clippy for `aarch64-apple-darwin` and Linux;
- `cargo deny` and the package list;
- the 19 `#[ignore]`d live tests;
- each of the 18 commits on its own.

An independent agent reviewed the FFI and the public API against XNU and found no memory-safety problem and no UB. He requests two changes before merge, plus nits to take now or after. Follow-ups and pre-existing issues are out of scope: `--filter` narrowing of the unreadable count, `watch --follow-new` after attach, and the `GpuQuerySource::Pdh` doc link.

Requested:
1. **Document the unresolved-template behaviour change.** When the `graphics_footprint` template index does not resolve, `tally_reads` returns `None` (`metal.rs:1106`) and `gpu_processes` falls through to `NoGpuSource`. On `main` the result was `Ok(vec![])`, so `hmn ps` goes from `0 GPU processes found.` with exit 0 to exit 2, unsandboxed included.
   - It is missing from R01's "Four behaviour changes are deliberate" (line 26) and from the CHANGELOG, and R01's byte-identical claim (line 127) does not hold on such a host.
   - Fix: a CHANGELOG *Changed* bullet, which also says `gpu_processes` can return `ProcessListDenied` where it returned `NoGpuSource` or an empty list, and a fifth R01 item.
2. **`list_kern_proc_all_holds_this_process` fails under Codex's policy**, which refuses `kern.proc.all`. The test is not `#[ignore]`d. Skip with a printed reason when the probe fails with `EPERM`, or route the test through `list_pids()`.

Nits:
- PID 0 is read before it is skipped.
- `denied_pids` is unsorted; the field doc should also say "always empty on Linux and Windows".
- `footprint_from_errno` maps every errno but `EPERM` to *gone*, so the silent empty list could return.
- A fail-closed listing reads as `NoGpuSource`, not a denial.
- `comm_to_name`'s valid-prefix rule applies at any length, not only at 16 bytes.
- CONVENTIONS: `// EXPLICIT:` on `tally_reads`, `// BORROW:` in new tests.
- R01's "in the same pass" (line 131) is false.

## Evidence
- Measured at 1c97d88 on the M3 Pro (stable 1.99.0, `rustup check` up to date). The lib test binary run under the campaign's pinned Codex profile (`codex.sb` at 6e7cf099e8) exits 101: 117 passed, 1 failed (`list_kern_proc_all_holds_this_process`). Unsandboxed, the same test passes.
- On the base b716087, `list_compute_processes` skipped `pid <= 0` before `read_graphics_footprint` and `continue`d on its `None`, so an unresolved index gave `Some(vec![])`.
- `grep -c 'pid <= 0' src/gpu/metal.rs` gives 1 (in `tally_reads`).
- 14 `.to_owned()`/`.as_bytes()` sites the PR added lack `// BORROW:`: 8 in `metal.rs` tests, 2 in `ps.rs` tests (c07282f) and 4 in `tests/macos_smoke.rs` `listing_label` (7e9e4aa).

## Root Cause
- `kinfo_enumeration` specified the four-outcome read and R01's design section, but not the matching *Why v0.2.14* list item or CHANGELOG entry for the template case.
- The `KERN_PROC_ALL` test was written for an unsandboxed `cargo test`.
- The nits are review-level refinements of code that passed its leaf gates.

## Impact Assessment
- **Scope:** `src/gpu/metal.rs`, `src/gpu/mod.rs`, `src/snapshot.rs`, `src/error.rs`, `src/bin/hmn/ps.rs` (tests), `tests/macos_smoke.rs`, `CHANGELOG.md`, `docs/roadmap-v0.2.14.md`.
- **Behaviour changes, macOS only:**
  - `denied_pids` is sorted ascending and deduplicated;
  - a listing in which no read worked and none was refused is `None`, then `NoGpuSource`, instead of `Ok(empty)`;
  - PID 0 is not read.
- **Windows and Linux** output is byte-identical.
- **History:** following the reminder (`sj-hypomnesis-pr8`), every fix is folded into the commit that owns it (`fixup!` + autosquash) and force-pushed, because the maintainer checked each commit on its own and merges with a merge commit. The 15 "at `6e7cf099e8`" mentions, all added by 107b567 after 6e7cf09, and that commit's message are repointed to the new hash of "docs(roadmap): record the consistency pass…" in one pass, as the last rewrite. The maintainer counted 14 mentions.

## Proposed Solution
Three idiom reviews (Sonnet, read-only, 2026-10-09) checked each fix against the maintainer's code and CONVENTIONS.md. Their results:
- **R2:** keep the `KERN_PROC_ALL` test. Read `last_errno()` as the first statement after the call; on `EPERM`, `writeln!(stderr)` a reason and return (libtest captures `eprintln!`; `cli_ps.rs` `accept` uses `writeln!`). There is no `panic!`, since `clippy::panic` is denied. `list_pids()` is rejected: under Codex it never reaches `KERN_PROC_ALL`, and it duplicates `list_pids_holds_this_process`.
- **PID 0:** `.filter(|&pid| pid > 0)` before the read, as `main` did. The guard in `tally_reads` goes.
- **The silent empty list:**
  - add `FootprintRead::Failed`, named after `LibprocPids::Failed`;
  - `Gone` means `ESRCH` only; any other errno, a missing one and the three `rc == 0` anomalies are `Failed` (the user's choice, 2026-10-09);
  - a local `failed` count in `tally_reads` gives `None` when nothing was read, nothing was denied and something failed.
- **Sorting:** `sort_unstable` + `dedup` in `decide_listing` before the count, and the field doc.
- **Fail-closed wording:** one sentence in the `NoGpuSource` variant doc; `Display` stays pinned.
- **`comm_to_name`:** the prefix rule applies only at `kinfo::P_COMM_SIZE - 1`.
- **CONVENTIONS:** one `// EXPLICIT:` above the loop; `// BORROW:` on all 14 PR-added sites (the user's choice).
- **Docs:** CHANGELOG *Changed* bullet and *Added* trim in d774660; R01 edits in 9bf9ab0.

The amendment adds three leaves:
- `review_code`: an implementer, then one adversarial verifier, since it touches measurement semantics and can fail silently.
- `review_docs`: written by the coordinator, prose reviewed by the user.
- `land/land_and_reply`: the coordinator autosquashes, runs the per-commit gates and repoints the hash. Then, with the user's yes each time, it pushes, waits for CI and replies.

## Recommendations
1. Approved by the user on 2026-10-09 (plan approval): the fixup-per-owning-commit history, `Failed` with the anomalies, and `// BORROW:` on all PR-added sites.
2. After the push and green CI, reply on the PR with a table, point by point, and the user re-requests review from the sidebar.
