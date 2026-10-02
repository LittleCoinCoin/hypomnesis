# Part 2 CLI

## Context
This level runs after gpu_process_listing. ps_watch_unreadable makes `hmn ps` and `hmn watch` report denied processes by count, with one platform remedy constant. `close/` follows with docs, the field check and the consistency pass.

## Goal
Turn the library's denial into the CLI's stated count, with a platform-correct remedy and exit codes that scripts can gate on.

## Pre-conditions
- [ ] gpu_process_listing `done` and merged into `v0214-part2`

## Success Gates
- ⬜ ps_watch_unreadable `done`, and its verifier's findings adjudicated

## Status
```mermaid
graph TD
    ps_watch_unreadable[Item 6: unreadable counts, remedy, exit status, watch notices]:::planned
    close[Part 2 close]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `ps_watch_unreadable.md` | 📄 Leaf Task | ⬜ Planned |
| `close/` | 📁 Directory | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
