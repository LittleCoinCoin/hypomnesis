# Part 2 public API

## Context
This level runs after kinfo_enumeration. gpu_process_listing exposes the macOS backend's denied PIDs through the new public `gpu_process_listing` and `HypomnesisError::ProcessListDenied`. `cli/` consumes them one level deeper.

## Goal
Add the additive public API under the names the maintainer fixed (R03), without changing `gpu_processes`' signature.

## Pre-conditions
- [ ] kinfo_enumeration `done` and merged into `v0214-part2`

## Success Gates
- ⬜ gpu_process_listing `done`, and its verifier's findings adjudicated

## Status
```mermaid
graph TD
    gpu_process_listing[Item 4: gpu_process_listing and ProcessListDenied]:::inprogress
    cli[Part 2 CLI]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `gpu_process_listing.md` | 📄 Leaf Task | 🔄 In Progress |
| `cli/` | 📁 Directory | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
