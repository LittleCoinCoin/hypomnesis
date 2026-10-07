# Gap Analysis: v0214-sandbox — PR C's leaves rebased onto main (A5)

## Problem Statement
PR B (mi-for-the-rust-of-us/hypomnesis#7) merged as b716087 on 2026-10-06, after the maintainer's review (A4). PR C's four leaves (`kinfo_enumeration`, `gpu_process_listing`, `ps_watch_unreadable`, `part2_close`) were last revised before that review. They name `v0214-part1`'s head as their base, predict PR B's behaviour "from its spec", and quote symbols, line numbers and test counts that PR B's review then changed. A5 rebases the spec, not the code: every statement in the four leaves is re-measured on b716087 and corrected before any implementer is dispatched.

## Evidence
Four sweepers, one per leaf, each in its own detached b716087 worktree, local `cargo` (stable 1.99.0 up to date, 1.88), `/usr/bin/sandbox-exec` unsandboxed (nested rc 0). Their edit tables, defect lists and every measuring command are in `10-a5_sweeps/` (`brief.md` is the shared brief; one file per leaf). Known drift, confirmed:

| # | Drift | Where on b716087 | Fixed in |
|---|---|---|---|
| 1 | `ENOMEM` already exists, un-cfg'd, used by `classify_kern_proc_pid`'s `Unusable` arm; its doc is true only for `KERN_PROC_PID` | `src/gpu/kinfo.rs:52-55` | kinfo_enumeration: reuse it; Step 2 rewrites its doc for both MIBs |
| 2 | `remedy_text(outside_sandbox, purpose: RemedyPurpose)`, `RemedyPurpose::{Names, Identify}` (42a0a32) | `src/bin/hmn/format.rs` | ps_watch_unreadable: every signature and call |
| 3 | Test idiom: `#[cfg]` statements and `#[cfg]` before `#[test]`, never `cfg!`; `#[ignore]` reasons start with "requires"; every test `#[allow]` must survive an `#[expect]` swap | `tests/smoke.rs:196-207`; `__reports__/v0214_part1/01-reviews_v0.md` | all four |
| 4 | Base is `PCfVW/main` b716087; gates that diffed against `v0214-part1` diff against the SHA | — | part2 README, gpu_process_listing, ps_watch_unreadable, part2_close |
| 5 | Line numbers and befores: every "PR B (from its spec)" is now `b716087 (measured)` | — | all four (31 + 26 + 40 + 24 edit sites) |

Further drift the sweep found:

- `sandbox.sh` on main reads options only before `PROFILE`; `sandbox.sh S --job -- …` runs `--job` as a command (exit 127), not a usage error. kinfo_enumeration Step 4 now specifies the parser change that recognises `--job` after the profile while an unknown profile still exits 64.
- The `#[ignore]`d `tests/macos_sandbox.rs` test was specified to print `skipped:` when it cannot sandbox. PR B's review (68007ba) requires failing instead, both inside a sandbox and with the child variable inherited. gpu_process_listing now mirrors `process_exists_under_sandbox_profiles` (probe profile measured: rc 0 unsandboxed, 71 under S, S0, P, L).
- metal.rs `mod tests` carries `#[allow(clippy::unwrap_used)]` only; PR B dropped `expect_used`.
- `CHANGELOG.md` `[Unreleased]` holds `### Changed` (3) and `### Fixed` (3), no `### Added`: the leaves that add entries create it.
- Lib tests on b716087: 95, natively and under Rosetta. `macos_smoke` ignored: 3. `format_ps_summary` filter: 28; `--bin hmn` 248 passed; `cli_ps` 2 passed, 1 ignored.
- `__reports__` on main: 145 tracked files and 31 `git grep` lines (CHANGELOG ×4, ROADMAP, the dogfooding report ×2, `src/gpu/kinfo.rs:32`, R01 ×23), all listed in part2_close Step 5.
- PR B's `ps_exit_code(exit_status, rows.is_empty(), failed)` call is pinned by grep (a run would need two GPUs). ps_watch_unreadable changes that call, so it gains its own grep pin for the new call.
- The App Sandbox gate ran the signed binary in place, which dies with SIGTRAP 133 under `~/Documents`; it now runs a copy from `mktemp -d /private/tmp/hmn-appsb.XXXXXX`, as part1_close's record does.

## Root Cause
The leaves were authored against `v0214-part1` before the review that changed PR B, and they predicted PR B's behaviour where it could not yet be measured. The campaign's own rule (GATE CONTRACT 6: quote, never paraphrase, the other leaf's gate) kept most cross-leaf quotes checkable, which is why the sweep could find each one.

## Impact Assessment
- Scope: prose of the four PR C leaves and the part2 README's Context and Pre-conditions. No node added, renamed or re-ordered; no BNF section edited by hand except this level's Amendment Log row.
- Design decisions taken by the coordinator (each in a leaf):
  1. **Record-size guard for `KERN_PROC_ALL`** (kinfo_enumeration). XNU (xnu-12377.1.9, `bsd/kern/kern_sysctl.c`): the size probe returns every record plus `KERN_PROCSLOP` (5 records); a short buffer gets the whole records that fit, then `ENOMEM` with `len` 0; `EPERM` leaves `len` as passed. Measured with a ctypes probe: 1061 probed against 1056 filled; a 3-record buffer comes back with 3 records, errno 12, `len` 0. A kernel whose `kinfo_proc` exceeds 648 bytes never answers `ENOMEM` to a probe-sized buffer, and the whole-records check catches it only when the total is not a multiple of 648 (about 1 in 81 for 656-byte records). So `trust_kinfo_listing(listing, self_lookup)` trusts a listing only when `kern_proc_pid_lookup(self)` is `PidLookup::Record`, which `classify_kern_proc_pid` grants only to exactly one 648-byte record; an empty `Records` is `Failed` too, as `libproc_outcome` treats an empty list. Two tests: kinfo_enumeration goes from 32 to 34, and part2_close's Rosetta arithmetic from +39 to +41 (95 → 136).
  2. **Same-boot base for `compare.py`** (kinfo_enumeration Step 4, part2_close Step 2). compare.py matches rows by PID, so the committed `pr_b/` cannot be the base after a reboot (measured: rc 1, PID 669 `ControlCenter` in `pr_b`, `Finder` now; two same-boot b716087 captures rc 0). Each leaf builds b716087 in a detached worktree under `target/base` and captures it immediately before its own capture.
  3. **`gpu_processes`' two lint attributes** suppress nothing on b716087 (an `#[expect]` swap is unfulfilled under no-default, default and all features). The one-line wrapper carries neither; `gpu_process_listing` carries only what an `#[expect]` swap shows it needs.
  4. **`sort_by_pid` gains `test` in its `cfg`**, because `decide_listing` is compiled under `cfg(test)`; a guard runs `cargo test --locked --no-default-features --lib --no-run`.
  5. **`Cargo.toml`'s `exclude = ["__reports__/"]` stays** after part2_close drops the directory.
  6. **New PR C Verification rows in R01 quote their evidence** rather than linking a `__reports__/` path.
- Residuals recorded, not fixed: the done leaf `remedy_macos` still says `"for names"` and its `Known residual:` grep gate expects 1 where main has 0 (A3 removed the paragraph). That is history; PR C's leaves quote main. The campaign README's c1810a5 baseline quotes the c1810a5 `NoGpuSource` text, which is correct for c1810a5.
- Risk: the sandbox test spec and `trust_kinfo_listing` are new design, not drift. They get the verifier's attack below and the independent FFI review during PR C.

## Proposed Solution
Commit the amended leaves to `v0214-roadmap` as one roadmap commit, then dispatch one Sonnet verifier to attack all four amended leaves for tautological or unpassable gates and for any quote that does not match main or the done PR B leaves. Adjudicate its findings in a follow-up roadmap commit before the first implementer is dispatched.

## Verification
One Sonnet verifier attacked the amended leaves (commit 0d1cbcd): PASS WITH NOTES, 0 blockers, 9 notes (`10-a5_sweeps/verify.md`). It re-ran a sample of befores on b716087 (all matched), the lib arithmetic (95 + 34 + 9 − 2 = 136), the XNU `KERN_PROC_ALL` behaviour (1097 probed against 1092 filled; a 3-record buffer: `rc` -1, errno 12, `len` 0) and the same-boot compare base. Adjudicated by the coordinator and applied in the follow-up roadmap commit: N1 (the `run_ps` pin's `catches:` narrowed to what a grep sees), N2 (the literal guard greps removed lines only), N3 (`Z --job` and `--job S` must exit 64), N4 (one `### Added` heading, counted in part2_close), N5 (a grep pin that `trust_kinfo_listing` is wired), N6 (`gpujob` compiled outside the sandbox into `target/`, `compare_names.py`'s prefix rule and empty-overlap exit, the harness README's PR C paragraph), N7 (live-list gates re-run once before a failure counts), N8 (the When-it-bites count scoped to its table). N9 needed nothing.

## Recommendations
1. Approved under the user's brief of 2026-10-07 ("Phase 0 … amendment A5, before any code").
2. Implementers measure, never trust, the `b716087 (measured)` values in their gates' befores; a mismatch is a spec defect to report, as in A4.
