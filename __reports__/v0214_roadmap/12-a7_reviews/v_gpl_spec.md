# v_gpl_spec: gpu_process_listing at c456939 (base 8519f1a)

Verdict: PASS WITH NOTES. 0 blockers, 1 should-fix (stale comment), 4 notes. Every Success Gate and
Consistency Check reproduced exactly as written; all Deliverables present.

## Findings
1. should-fix, tests/smoke.rs:155-157. The comment of `gpu_processes_returns_result_or_no_gpu_source`
   still says "on macOS only NoGpuSource is accepted"; the macOS arm below now also accepts
   `ProcessListDenied`. OWNERSHIP says every stale comment is updated in the leaf. Fix: "on macOS
   NoGpuSource or ProcessListDenied are accepted".
2. note, tests/macos_smoke.rs:120-128. `gpu_processes_returns_metal_rows_for_self` still says "we
   accept three outcomes ... (c) Err(NoGpuSource) on a non-GPU host"; the Err arm also accepts
   `ProcessListDenied` (sandboxed). Add it to (c).
3. note, src/error.rs (ProcessListDenied variant doc). "when the caller's sandbox refuses the ledger
   read of every process but its own" is slightly narrower than the code (`others_read == 0 &&
   denied > 0`): under profile L the caller's own read is refused too. The rustdoc table in
   gpu/mod.rs ("none other than the caller's could be read") is exact; align the variant doc to it.
4. note, tests/macos_smoke.rs:3-6. The module-doc hard wrap is now uneven after inserting
   `gpu_process_listing`; reflow the paragraph.
5. note, commit message. The body of da3d698 does not mention tests/macos_sandbox.rs (only 38c151b's
   does, and 38c151b is squashed away). Mention it in the squashed message.

## Gates (all exact values)
- process_list_denied: 5 listed, 5 passed; decide_listing: 3/3; process_list_denied_display: 1/1;
  signature guard: 1/1; Step 2 list of 9: 9.
- doctests: `1` and `1`.
- macos_sandbox: `1`, `exit 0`, `5`, `inherited exit 101 1`, `sandboxed exit 101 1`. LISTING lines:
  S ok denied=1013 caller_denied=false overlap=false; S0/P/L err=ProcessListDenied denied=1013;
  unsandboxed ok denied=0 entries=24 caller_denied=false overlap=false.
- CLI (release build): P/S0/L `ps --device 0` -> `P 2 1 1 0`, `S0 2 1 1 0`, `L 2 1 1 0`;
  `ps` -> `S0 2 1 1`, `L 2 1 1`; profile S -> `exit 0 skipped=0 found=1 selfrow=1`.
- Guard under profile P: `smoke exit 0`, `macos_smoke exit 0`, ProcessListDenied counts 4 and 3.
- Static: four=0 five=1, lib.rs gpu_process_listing 1 / GpuProcessListing 1, legacy_entries 0,
  CHANGELOG 2, listing_without_denials( 4, backend-file diff vs b716087 = 0,
  `cargo check --all-features --target x86_64-unknown-linux-gnu --tests` rc 0, both no-default
  feature checks `rc=0 warnings=0`, `cargo test --no-default-features --lib --no-run` rc 0.
- Step 1 (38c151b) expected FAIL: 9 `panicked at`, 0 `error[E`; signature guard passes.
- Steps 2 and 3 consistency checks: rc 0.
- Shared gate set, da3d698 and c456939 each on its own: fmt, clippy default / all-features / linux
  target, test --all-features (145 lib + 248 bin, 6 passed 3 ignored in macos_smoke), doc -D warnings,
  +1.88 check, no-default check, nvml/dxgi/pdh check, +1.88 clippy and test, release build: all rc 0,
  0 warnings. Rosetta (x86_64-apple-darwin) lib tests on the tip: 145 passed.
- Extra feature sets (metal, nvml, nvidia-smi-fallback, pdh, dxgi, metal+smi) clippy --all-targets
  on macOS and on the linux target: all rc 0.
- `#[allow(clippy::expect_used)]` swapped for `#[expect]` in tests/macos_sandbox.rs: clean under
  stable default, stable all-features, 1.88: all three allows are needed.
- `cargo package --list --allow-dirty --locked`: 81 files, 0 from `__reports__/` (Cargo.toml
  excludes it); tests/macos_sandbox.rs is listed.

## Mutations (each in a scratch worktree, reverted)
- process_list_denied always None: caught by `..counts_refusals..`, `..saturates..`,
  `decide_listing_is_denied..` and the macos_sandbox test.
- any denial is an error: caught by `..is_none_when_another_process_was_read`,
  `decide_listing_sorts..`, macos_sandbox (profile S).
- `others_read <= 1`: caught by the same two lib tests and macos_sandbox.
- decide_listing unsorted: caught by `decide_listing_sorts_by_pid..` only (as designed).
- smoke.rs without ProcessListDenied under profile P: exit 101, `unexpected error ... ProcessListDenied`.
- `test` dropped from sort_by_pid cfg: E0425 in `--no-default-features --lib --no-run`.
- gpu_processes returning the listing: E0308 in `cargo check --tests`.

## Checked and sound
- Public API is additive only: new `gpu_process_listing`, `GpuProcessListing` (`#[non_exhaustive]`,
  `Debug, Clone`), `HypomnesisError::ProcessListDenied { denied: u32 }` (enum still
  `#[non_exhaustive]`), and two re-exports. No signature changed, nothing removed (the only
  removed pub-looking item is the `pub(super)` `list_compute_processes`).
- Rustdoc vs code: the per-platform table matches each arm. `GpuProcessListing` literals exist only
  in `decide_listing` (macOS) and `listing_without_denials` (empty Vec), so Linux and Windows are
  empty by construction. `decide_listing` drops the caller's row on `Err`. `Display` carries no
  remedy word (test pins re-run, sandbox, elevated, sudo).
- Seam: `decide_listing(list.entries, list.denied_pids, list.others_read)` receives
  `list_processes`'s fields untouched; only `entries` is sorted by PID. `tally_reads` guarantees the
  caller, gone and non-positive PIDs are not in `denied_pids`, matching the doc.
- macos_sandbox test fails (exit 101, message asserted) when run inside a sandbox or with
  `HMN_GPL_CHILD` inherited; runs S, S0, P, L and unsandboxed (5 LISTING lines); a child that
  panics prints no LISTING line, so the parent fails.
- Deviations: test number 9 is correct (tests in file order: 1 rss, 2 count, 3 info, 4
  process_gpu_info, 5 gpu_processes, 6 snapshot, 7 out-of-range, 8 process_exists sandbox, 9 new;
  `ignored` count stays 3). `row()` removed from metal.rs is harmless (only the deleted
  legacy_entries tests used it). `pub(super)` on `list_processes`, `MetalProcessList` and fields is
  required by the Metal arm; the `None` doc moved with it and no behaviour changed. `# Errors` on
  `gpu_processes` in Step 2 is needed (clippy `missing_errors_doc` is warn, -D warnings, so da3d698
  must carry it to be green on its own); Step 3 only expands the prose.
- OWNERSHIP: dispatcher count, module doc, error.rs structured-fields doc, GpuProcessEntry doc,
  gpu_processes source-priority list, Metal-arm comment all updated; CHANGELOG `[Unreleased]` holds
  exactly one `### Added` plus the Changed entry, in final-state voice. Remaining stale text is the
  two test comments above. README limitation 9 and the CLI texts belong to part2_close and the CLI
  leaf.

## BLOCKED
- `cargo deny`: not installed here (`cargo --list | grep -c deny` prints 0); CI job only.
- Running `gpu_process_listing_has_no_denied_pids_off_macos` on Linux/Windows: compiled for Linux
  (`cargo check --target x86_64-unknown-linux-gnu --tests` rc 0, clippy rc 0); run only in CI. The
  Windows target is not installed.
