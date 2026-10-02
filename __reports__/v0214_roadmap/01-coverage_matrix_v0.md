# Coverage matrix — v0214-sandbox roadmap (R08), amended after verification round 0

Every R01 scope item, every point of the maintainer's reply (R03, comment 5947395182), and every
code or doc site from the Phase 0 code-site map is mapped to the leaf and step that owns it. The
drafting agents supplied the mappings; the coordinator assembled them. The two integrity verifiers
re-derive this list independently and attack it.

Leaf paths are relative to `__roadmap__/v0214-sandbox/`. "c1810a5" is the measurement baseline. It
is behaviour-identical to the rebased planning base 40a701e: 587a6d5 changes test assertions only.

## R01 scope items

| # | Item | Owner |
|---|---|---|
| 1 | Metal arm in `bounds_check`; Metal named in `NoGpuSource` and `# Errors` | `part1/metal_bounds_check` 1–2 |
| 2 | `process_exists` lookup rule, PID 0, sandboxed callers | `part1/process_exists_kinfo` 1–4 |
| 3 | `KERN_PROC_ALL` enumeration, `p_comm` names on EPERM, four-outcome read | `part1/close/part2/kinfo_enumeration` 1–4 |
| 4 | `gpu_process_listing`, `denied_pids`, `ProcessListDenied` | `…/part2/api/gpu_process_listing` 1–3 |
| 5 | `hmn ps` skip line; exit 2 when every tried device failed | `part1/ps_failed_devices` 1–3 |
| 6 | `unreadable` counts, remedy constant, `--exit-status` 2, `watch` notices | remedy text pulled forward to `part1/remedy_macos` (U1); the rest in `…/part2/api/cli/ps_watch_unreadable` 1–4 |
| 5′ | `--exit-status` with a partial device failure and nothing listed → 2 (U2, an R01 amendment) | `part1/ps_failed_devices`; carried by `ps_watch_unreadable`'s `ps_exit_code`; recorded in R01 by `docs_pr` 1 |
| 7 | `n/a` vs `?` in the SPILL and PAGED cells | `part1/spill_cell_na` 1–3 |
| 8 | macOS limitation restated, one canonical statement | `part1/close/part1_close` 1–2 (PR B form); `…/cli/close/part2_close` 1 (PR C rewrite) |
| 9 | `spilled: null` notice and ROADMAP v0.3.0 entries | notice already written; ROADMAP entries in `docs_pr` 3 |
| 10 | Field report F5 corrected | already done (commit 7358111) |
| 11 | README, FAQ, tutorials, CHANGELOG, ROADMAP | each code leaf's own lines (OWNERSHIP rule); the rest in `part1_close` 1–2 and `part2_close` 1 |

## Maintainer reply (R03)

| Point | Owner |
|---|---|
| Docs-only PR A first (report, plan, `__reports__/`) | `docs_pr` 1–4 |
| Part 1 = items 1, 2, 5, 7, 8 | `part1/` leaves |
| Part 2 = items 3, 4, 6, with adversarial review | `part2/` leaves, each `[V]` |
| API names `gpu_process_listing`, `GpuProcessListing { entries, denied_pids }`, `ProcessListDenied { denied }` | `gpu_process_listing` 1–2 |
| Remedy `N unreadable — re-run outside the sandbox`, mirroring Windows | `ps_watch_unreadable` 1–2 |
| Library docs weighted on the count ("no outside" in an App Sandbox) | `gpu_process_listing` 2 (per-platform doc table); recorded as a decision in `docs_pr` 1 |
| `spilled: null` waits for v0.3.0 | `docs_pr` 1 (decision) and 3 (ROADMAP entry) |
| `kinfo_proc`: whole-records check + architectures verified | `docs_pr` 1 (plan note, evidence `kinfo_proc_layout.md`); `process_exists_kinfo` 2–3 (code doc, Rosetta test); `part1_close` flips the Rosetta sub-bullet to past tense |
| CI: exit 2 and `tests/cli_ps.rs` seen passing on macos-latest | `ps_failed_devices` 2–3; PR-level gates in `part1/README` and `part2/README` |
| When-it-bites row for Claude Code's sandbox | `docs_pr` 1 (row) and 2 (0.2.13 measured, user in the loop); `part1_close` 4 and `part2_close` 2 (re-measured) |
| Drop `__reports__/` at release, as `f3c6010` | `part2_close` 5 |

## Code and doc sites (from the code-site map, corrected by the drafters)

| Site | Owner |
|---|---|
| `gpu/mod.rs` `bounds_check` and its doc; `# Errors` of `device_count`, `device_info`, `process_gpu_info`, `gpu_processes` | `metal_bounds_check` 2 |
| `error.rs` `NoGpuSource` text and doc; enum doc "Apple Metal" as future | `metal_bounds_check` 2 |
| `tests/smoke.rs`, `tests/macos_smoke.rs`: accepted errors tightened (item 1) | `metal_bounds_check` 1 |
| the same tests widened to accept `ProcessListDenied` (sandboxed `cargo test`) | `gpu_process_listing` (drafter's defect 2) |
| `metal.rs` `process_exists`; `libsystem_ffi::sysctl`; new `src/gpu/kinfo.rs` | `process_exists_kinfo` 1–3 |
| `gpu/mod.rs` `process_exists` tests (macOS arms) and doc table row | `process_exists_kinfo` 1, 4 |
| `CONVENTIONS.md` macOS syscall list (`KERN_PROC_PID`) | `process_exists_kinfo` 4 |
| README ~:521 capability table, "Process existence" macOS cell (stale after item 2; found by the PR B fixer) | `process_exists_kinfo` 4 |
| `CONVENTIONS.md` (`KERN_PROC_ALL`) | `kinfo_enumeration` 3 (C-2) |
| `ps.rs` `run_ps` skip/exit; `run_ps` doc; `main.rs` `--device`/`--exit-status` help; README `--device` line; `tests/cli_ps.rs` | `ps_failed_devices` 1–3 |
| `format.rs` `spill_cell` → `spill_cell_for`/`paged_cell_for`; `watch.rs` PAGED; the ps/watch tests pinning `?` (including the 3 extra the drafter found) | `spill_cell_na` 1–2 |
| `main.rs` ~132 spill help; FAQ:206/210/246; README Limitations bullet 4 (~:270, not :449); tutorial `watching-a-running-job` ~:50 | `spill_cell_na` 3 |
| README:285, 293, 520, 523; `lib.rs`:19, 21; `main.rs`:16-18, 148, 169-171, 194-197; `ps.rs`:508; `mod.rs`:374; nine `metal.rs` doc sites; FAQ `sudo`/cross-user passage; ROADMAP Principle 4 and Intel row; `smoke.rs` comment | `part1_close` 1–2 |
| README:58 | untouched: makes no false claim (drafter verified) |
| `metal.rs` doc sites rewritten again for PR C behaviour | `kinfo_enumeration` 2 |
| `read_graphics_footprint` four outcomes, EPERM const, `list_compute_processes` | `kinfo_enumeration` 1–2 |
| `snapshot.rs` `GpuProcessEntry::name` truncation sentence; FAQ; `--filter` help (ps and watch) | `kinfo_enumeration` 3 |
| `gpu/mod.rs` `gpu_processes` → wrapper; module doc "five dispatchers"; `lib.rs:62` re-export; `snapshot.rs` `GpuProcessListing`; `error.rs` `ProcessListDenied` | `gpu_process_listing` 1–3 |
| `ps.rs`:586 and `watch.rs`:351 "re-run elevated" literals → `format::REMEDY_OUTSIDE_SANDBOX` | `part1/remedy_macos` (U1); `ps_watch_unreadable` adds the unreadable clause |
| `SummaryNotes.unreadable`; `judge`/`--pid` on denied PIDs; `--exit-status` 2 | `ps_watch_unreadable` 1–2 |
| `watch.rs` `missing_pid_notices`, "found no GPU processes", follow-new, attach notices, growth hint | `ps_watch_unreadable` 1, 3 |
| `main.rs` `after_long_help` macOS bullet (PR C form) | `ps_watch_unreadable` 2 |
| `main.rs` ~150 `--help` Security note ("bare `?` on Linux/macOS … under elevation") and its FAQ twin (~289) | `part1_close` 1–2 (B-6) |
| README:258 "summary is always printed", `ps.rs` "Always printed" doc, README:201 exit codes | `ps_failed_devices` (B-4) |
| FAQ / `snapshot.rs` sentence "a macOS name is `?` only when `proc_pidpath` is withheld" (false after PR C) | `kinfo_enumeration` 3 (C-2); the `format_ps_summary` doc sentence is owned by `ps_watch_unreadable` (C-4) |
| FAQ self-denying-profile line (Foundation `libdispatch` abort) | `part1_close` only (B-6); removed from `part2_close` |
| Sandbox harness `__reports__/v0214_part1/harness/` (profiles P, Q, S, S0, D, L, C; `compare.sh <base> <new> [--spill-map]`) | `part1_close` (G11); extended by `kinfo_enumeration` 4 |
| Ledger-only denial (profile L, R01's silent-skip path) | residual stated in `part1_close`; fixed and gated in `ps_watch_unreadable` and `part2_close` |
| Codex Seatbelt policy copy, pinned to an openai/codex commit | `part1_close` (G11) |
| CI branch record (which `cli_ps` acceptance fired on macos-latest) | `ps_failed_devices` prints it; `part1_close` records it from PR B's CI log |
| R01 amendments for U1, U2, U3; CI note; the Verification P line in the PR B and PR C forms | `docs_pr` 1 (A-6) |
| tutorials `is-my-run-spilling.md`:38 | `part2_close` 1 (edited only if it names macOS) |
| README banner; `Cargo.toml` version | the maintainer's release commit (not in this campaign) |
| R01 Verification and scope statuses | `part1_close` 5 (items 1, 2, 5, 7, 8); `part2_close` 3–4 (items 3, 4, 6, 11) |

## Adjudicated

The three open items of round 0 were decided by the user (U1, U2) and the coordinator (`KERN_PROC_ALL` row). See `00-roadmap_verification_v0.md`.
