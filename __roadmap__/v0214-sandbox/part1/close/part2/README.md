# PR C: part 2, measuring inside a sandbox

## Context
PR C, head `v0214-part2`, branched from PR B's head and rebased onto `main` as PR B merges. It carries R01 items 3, 4 and 6, plus the rest of item 11. Each item is a sequential level, because each consumes the previous one's output: item 3's four-outcome read and denied PIDs, then item 4's public `GpuProcessListing`, then item 6's CLI counts. All three get adversarial review (R03).

## Goal
Measure everything a sandbox permits, state what it forbids by count, and stop reporting an unreadable list as an empty one.

## Pre-conditions
- [ ] part1_close `done`, with its gate `find __reports__/v0214_part1/pr_b __reports__/v0214_part1/c1810a5 -name '*.exit' | wc -l` → 36 met (the baselines PR C compares against)
- [ ] Branch `v0214-part2` exists at `v0214-part1`'s head

## Success Gates
- ⬜ kinfo_enumeration, gpu_process_listing, ps_watch_unreadable and part2_close `done` in `dirtree-rdm ls`
- ⬜ `gh pr checks <PR C>`: every job green, including both macos-latest jobs, with tests/cli_ps.rs passing there
- ⬜ Coordinator seam review recorded: the denied PIDs kinfo_enumeration collects are the ones gpu_process_listing exposes, and the ones ps_watch_unreadable counts, with no PID counted twice or dropped

## Gotchas
Every new behaviour is macOS-only by construction. Windows and Linux output stay byte-identical, which pinned tests prove (for example the ps.rs "re-run elevated" tests), not a claim.
`spilled: null` is NOT in this PR (R03, Principle 2).

## Status
```mermaid
graph TD
    kinfo_enumeration[Item 3: sysctl enumeration and four-outcome read]:::planned
    api[Part 2 public API]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `kinfo_enumeration.md` | 📄 Leaf Task | ⬜ Planned |
| `api/` | 📁 Directory | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
