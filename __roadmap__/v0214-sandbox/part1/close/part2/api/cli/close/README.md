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
    part2_close[Item 11 rest, field check, consistency pass]:::done
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `part2_close.md` | 📄 Leaf Task | ✅ Done |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
| `part2_close.md` | `task/part2_close`, then `task/consistency` | a8a8644, 18abc98, 9244cf6 (Steps 1–3), 10c6e57, 4f7a0dc (consistency rows), 6e7cf09 (Step 4), 107b567 (Step 5), 1c97d88 (final notes), on c026a93 | Sonnet implementer for Steps 1–3: README item 9 "measures what is permitted and counts the rest", FAQ and `--help` stale `?` sentence fixed in three places, field check (seam hmn=947 probe=947; App Sandbox from `/private/tmp`; Claude Code row measured by the user's sandboxed session: `full`; CI run 37650659228 `branch=expected` ×4). Step 4 redefined by the user as a consistency pass against the maintainer's idiom: a fresh-eyes Sonnet reviewer found 21 rows (`__reports__/v0214_roadmap/16-part2_close/consistency_pass.md`); the user approved 1–7, 9 (the `Display` shape `process list unreadable (…)`), 10, 14, 16, 17, applied by a Sonnet implementer; coordinator equivalence run (528 cells): only the `Display` text differs. Step 5 by the coordinator: 370 files and 31 doc lines dropped, every mention named at `6e7cf099e8`. One Sonnet verifier: PASS WITH NOTES, 0 blockers; its notes fixed in 1c97d88, its spec defects (the old `Display` pattern in three gates) fixed in the leaf. Full gate set green on every commit. |
