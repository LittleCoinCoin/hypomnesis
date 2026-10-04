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
    metal_bounds_check[Item 1: Metal arm in bounds_check]:::inprogress
    process_exists_kinfo[Item 2: process_exists through kinfo_proc]:::inprogress
    ps_failed_devices[Item 5: ps states skipped devices, exits 2 when all failed]:::inprogress
    spill_cell_na[Item 7: n/a vs ? in SPILL and PAGED cells]:::inprogress
    close[Part 1 close]:::inprogress
    remedy_macos[Item 6 pulled forward: macOS remedy text]:::inprogress
    maintainer_review_a[A1: PR A review asks — package exclude, remedy split, toolchain freshness]:::amendment
    test_review_b[A2: test review — strength, idiom, CONVENTIONS]:::amendment
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `metal_bounds_check.md` | 📄 Leaf Task | 🔄 In Progress |
| `process_exists_kinfo.md` | 📄 Leaf Task | 🔄 In Progress |
| `ps_failed_devices.md` | 📄 Leaf Task | 🔄 In Progress |
| `spill_cell_na.md` | 📄 Leaf Task | 🔄 In Progress |
| `close/` | 📁 Directory | 🔄 In Progress |
| `remedy_macos.md` | 📄 Leaf Task | 🔄 In Progress |
| `maintainer_review_a.md` | 📄 Leaf Task | 🔵 Amendment |
| `test_review_b.md` | 📄 Leaf Task | 🔵 Amendment |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|
| A1 | 2026-10-04 | `__reports__/v0214_roadmap/06-amendment_a1_maintainer_review_v0.md` | ["maintainer_review_a.md"] | PR A review (merged 1bb9b22) asks PR B for the `__reports__/` package exclude, the remedy-split sentence, a toolchain-freshness gate and a stale-link fix |
| A2 | 2026-10-04 | `__reports__/v0214_roadmap/07-amendment_a2_test_review_v0.md` | ["test_review_b.md"] | Two read-only test reviews found a vacuous hardware test, weak and tautological tests, an unneeded lint allow and foreign idiom; fixes fold into the leaf commits before a force-push |

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
