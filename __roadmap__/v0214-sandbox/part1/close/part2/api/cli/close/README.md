# Part 2 close

## Context
This is the last level of the campaign. part2_close rewrites the canonical limitation statement for PR C's behaviour, runs every *When it bites* row on hardware, makes a fresh-eyes consistency pass, and drops `__reports__/` as `f3c6010` did.

## Goal
Leave `v0214-part2` release-ready: docs true, every fixture re-run, and no development artifacts in the tree.

## Pre-conditions
- [ ] ps_watch_unreadable `done` and merged into `v0214-part2`

## Success Gates
- ⬜ part2_close `done`, and its verifier's findings adjudicated
- ⬜ `git ls-files __reports__ | wc -l` on `v0214-part2` → 0

## Status
```mermaid
graph TD
    part2_close[Item 11 rest, field check, consistency pass]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `part2_close.md` | 📄 Leaf Task | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
