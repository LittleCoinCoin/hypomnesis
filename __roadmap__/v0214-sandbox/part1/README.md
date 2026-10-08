# PR B: part 1, cross-platform fixes and docs

## Context
PR B, head `v0214-part1`, branched from PR A's head and rebased onto `main` once PR A merges. It carries R01 scope items 1, 2, 5, 7 and 8, with their share of item 11, plus the macOS remedy text pulled forward from item 6 (user decision U1).
The five code leaves at this level run in parallel. Their edits are non-adjacent, and the coordinator resolves the textual conflicts in ps.rs, format.rs, tests/macos_smoke.rs and CHANGELOG at rebase. `close/` then restates the macOS limitation and field-checks the PR on hardware.
Item 2's `kinfo_proc` parser is a contract that PR C builds on.

## Goal
Correct the statements that are wrong today and the silent wrong answers that exist on every platform, without new public API.

## Pre-conditions
- [ ] docs_pr gate "PR A is open and CI is green" met, so R01 in the tree records the PR split and the maintainer's decisions
- [ ] Branch `v0214-part1` exists at PR A's head (or at `main` once PR A merged)

## Success Gates
- ⬜ All seven leaves at this level (the five code leaves, amendment A1's maintainer_review_a and amendment A2's test_review_b) and part1_close in close/ are `done` in `dirtree-rdm ls`
- ⬜ `gh pr checks <PR B>`: every job green, including both macos-latest jobs (R03: the exit-2 change must be seen passing on the VM runners, not assumed)
- ⬜ Coordinator seam review recorded in the Progress notes: process_exists_kinfo's parser API is the one kinfo_enumeration's leaf text names, and part1_close's canonical statement describes PR B's behaviour (exit 2 plus a skip line under the report's profile), not PR C's

## Gotchas
No public API is added in PR B. `process_exists` keeps its signature, and its macOS behaviour changes only where it answered wrongly (PID 0) or not at all (sandboxed callers).
PR B goes up as a DRAFT PR (with the user's yes for the push and the PR) as soon as the five code leaves are merged into `v0214-part1`, before part1_close runs, so that macos-latest CI evidence exists for ps_failed_devices' CI gate and part1_close's CI-branch record. It is marked ready for review after part1_close. ps_failed_devices' CI gate can therefore only be checked once that draft push has run CI, which is after the five code leaves are merged. ps_failed_devices is marked `done` at that point, not at merge.
The canonical limitation statement must not promise "measures what is permitted and counts the rest". That is PR C's behaviour. If PR B ships as a release on its own, the docs must be true for it, and so must its runtime hints: remedy_macos removes the macOS "re-run elevated" advice. If PR C never ships, `__reports__/` is dropped by a follow-up before release.

## Status
```mermaid
graph TD
    metal_bounds_check[Item 1: Metal arm in bounds_check]:::done
    process_exists_kinfo[Item 2: process_exists through kinfo_proc]:::done
    ps_failed_devices[Item 5: ps states skipped devices, exits 2 when all failed]:::done
    spill_cell_na[Item 7: n/a vs ? in SPILL and PAGED cells]:::done
    close[Part 1 close]:::done
    remedy_macos[Item 6 pulled forward: macOS remedy text]:::done
    maintainer_review_a[A1: PR A review asks — package exclude, remedy split, toolchain freshness]:::done
    test_review_b[A2: test review — strength, idiom, CONVENTIONS]:::done
    maintainer_review_b[A4: PR B review — ENOMEM, process ownership, nits]:::done
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `metal_bounds_check.md` | 📄 Leaf Task | ✅ Done |
| `process_exists_kinfo.md` | 📄 Leaf Task | ✅ Done |
| `ps_failed_devices.md` | 📄 Leaf Task | ✅ Done |
| `spill_cell_na.md` | 📄 Leaf Task | ✅ Done |
| `close/` | 📁 Directory | ✅ Done |
| `remedy_macos.md` | 📄 Leaf Task | ✅ Done |
| `maintainer_review_a.md` | 📄 Leaf Task | ✅ Done |
| `test_review_b.md` | 📄 Leaf Task | ✅ Done |
| `maintainer_review_b.md` | 📄 Leaf Task | ✅ Done |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|
| A1 | 2026-10-04 | `__reports__/v0214_roadmap/06-amendment_a1_maintainer_review_v0.md` | ["maintainer_review_a.md"] | PR A review (merged 1bb9b22) asks PR B for the `__reports__/` package exclude, the remedy-split sentence, a toolchain-freshness gate and a stale-link fix |
| A2 | 2026-10-04 | `__reports__/v0214_roadmap/07-amendment_a2_test_review_v0.md` | ["test_review_b.md"] | Two read-only test reviews found a vacuous hardware test, weak and tautological tests, an unneeded lint allow and foreign idiom; fixes fold into the leaf commits before a force-push |
| A4 | 2026-10-06 | `__reports__/v0214_roadmap/09-amendment_a4_pr_b_review_v0.md` | ["maintainer_review_b.md"] | Maintainer's CHANGES_REQUESTED on PR B: ENOMEM reads as can't tell, process (not file) ownership, six nits including a `RemedyPurpose` enum |

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
| `close/` | `v0214-part1`, `v0214-part2` | see close/README.md | part1_close (PR B, merged as b716087) and part2/ (PR C, #8, ready for review). |
| `maintainer_review_b.md` | `task/maintainer_review_b` | 6de108d, cbb7aeb, 42a0a32, d2d26bb, 68007ba, 0a685fb (on PR B, fast-forward from 4d32647) | A4, the maintainer's CHANGES_REQUESTED of 2026-10-05: `ENOMEM` → `Unusable` (named const, `Refused` not narrowed), process ownership at five sites, all six nits incl. `RemedyPurpose` (domain-named after `SortKey`/`PsJudgement`, user's choice over the review's `Purpose`). Three of the coordinator's recorded gate values were wrong (`--help` count 1 → 2, remedy-string before 3 → 8, `cargo doc` before only with `-D warnings`); the implementer stopped or reported each, and the gates were corrected. Sonnet verifier PASS WITH NOTES (three doc notes adopted). Test review: 21 mutants, 19 killed; the two survivors (narrowing `Refused` to `EPERM`, `errno` read when `rc == 0`) are now killed by two added assertions; one stale test name corrected. Coverage review: COMPLETE; the `kern.proc`-hiding residual written in `process_exists`'s rustdoc, the macOS remedy rule stated once, the review record's errno sentence corrected. Every pushed commit green on the full gate set (72/72). Reply posted with the user's validation as a table-led comment (issuecomment-6006287998), including the enum-naming note. |
| `part1/` (seam review) | `v0214-part1` | PR B head 4d32647 | Coordinator seam review: process_exists_kinfo's parser API (`KINFO_PROC_SIZE`, `P_PID_OFFSET`, `P_COMM_OFFSET`, `P_COMM_SIZE`, `ESRCH`, metal-only `CTL_KERN`/`KERN_PROC`/`KERN_PROC_PID`, `KinfoRecord`, `PathLookup`, `PidLookup`, `parse_kinfo_records`, `classify_kern_proc_pid`, `decide_exists`) is the one kinfo_enumeration's leaf names (verifier-checked); `remedy_text`/`REMEDY_OUTSIDE_SANDBOX`, `device_query_failure_line`, `ps_exit_code`, `accept(label, code, stderr, expected)` keep the names PR C extends; part1_close's canonical statement describes PR B (exit 2 plus the skip line under the report's profile, the `ledger`-only residual) and promises nothing of PR C (verifier seam check PASS). Harness entry points for PR C: `sandbox.sh`, `python3 capture.py`, `python3 compare.py` (A3). |
| `metal_bounds_check.md` | `task/metal_bounds_check` | dde079c (on PR B, mi-for-the-rust-of-us/hypomnesis#7; red f94c9e8 squashed into 98ebcde, A2 Step 1 folded in) | All gates PASS on the implementer's report (no verifier, as decided). macOS `NoGpuSource` text now pinned byte for byte and `bounds_check` edges pinned unsandboxed by A2. Linux/Windows leg of the error test ran first on PR B's CI (run 37210951264, green). |
| `process_exists_kinfo.md` | `task/process_exists_kinfo` | dde1bf9, e0125b9 (Steps 1–3 squashed; A2 Step 5 folded in) | Sonnet verifier PASS: re-ran every gate, `// SAFETY:` and errno order sound, 9 mutants of the parser/decision rule each caught (the `pid == 0` one only by the `#[ignore]`d sandbox table). Verifier's low finding adjudicated by the coordinator: a zombie now reads `Some(true)` (was `Some(false)`), documented in the `process_exists` rustdoc and CHANGELOG at integration. Deviations accepted: full architecture statement in the module doc only; CONVENTIONS row renames `sysctl` → `sysctlbyname` beside the MIB form. Seam: kinfo API names/visibilities match the Step 2 contract `kinfo_enumeration` cites (verifier-checked). |
| `ps_failed_devices.md` | `task/ps_failed_devices` | 4495484, 3e7f5d2 (red 8897a01 squashed; A2 Step 3 folded in) | Sonnet verifier PASS, maintainer's #6 rule confirmed (exit 2 only with a `(skipped)` line, both cli_ps tests). CI gate met on PR B run 37210951264: both macos-latest jobs SUCCESS, 4 macOS `cli_ps: … branch=expected` lines (with and without `--exit-status`), 0 `rejected`: the macOS VMs took the old exit codes, never the exit-2 path. Partial failure not CLI-testable on one GPU (PR body says so). |
| `spill_cell_na.md` | `task/spill_cell_na` | 2fb515a, 453ed2e (red 931b5b3 squashed; A2 Step 2 folded in) | All gates PASS on the implementer's report. A2 replaced `if cfg!(windows)` with `#[cfg]` constants and removed the two redundant alignment tests, so the Step 2 filter counts 20, not 21 (recorded in A2). |
| `remedy_macos.md` | `task/remedy_macos` | 1b24810 (red 5ddc6aa squashed; A2 Step 4 folded in) | Sonnet verifier PASS (8 mutants; the `process_sample` call site is pinned by grep only, PR body says so). CHANGELOG bullet restyled by the coordinator to the house format at integration. A2 deleted the tautological `remedy_outside_sandbox_is_the_macos_compile_time_platform`; the seam test PR C cites is kept. |
| `maintainer_review_a.md` | `task/maintainer_review_a` | ef24b99, 76e47f9, fdf2f6c | A1: `cargo package --list` `__reports__/` 18 → 0 (98 → 80 files), R01 remedy-split sentence, `rustup check` freshness gate (also in the campaign gate set and the per-commit gate script), stale findings link fixed. No verifier (mechanical). |
| `test_review_b.md` | `task/test_review_b` | folded into the five leaf commits above (90ca43d, 986d501, 6e2f423, edff7ef, 55a0d30 before folding) | A2: two read-only test reviews (library, CLI) with mutations and `#[allow]`→`#[expect]` checks; 11 fixes applied, every mutation now killed. Deviation accepted: the sandbox-table probe uses a profile unique to the file, since macOS lets a sandboxed process re-apply its own profile. 11 folded commits × 12 gates = 132/132 green before the force-push (807cd2f → fdf2f6c, user's yes). |
