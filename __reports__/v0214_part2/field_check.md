# part2_close: the When-it-bites rows run on hardware

Host: Apple M3 Pro, macOS 26.6.2, arm64, stable Rust 1.99.0 (`rustup check`: up to date). Run on 2026-10-08, from the repository root of the `task/part2_close` worktree.

Binaries: `cargo build --release --locked` (default features, so no `[nvidia-smi debug]` line) of the tree at a8a8644 (the Step 1 commit; it differs from PR C's head c026a93 by documentation and rustdoc only), and of b716087 (PR B merged, PR C's base), built in a detached worktree under `target/base` in the same boot as the captures and removed afterwards. `H` is the release `hmn` of the tree, `K` is `__reports__/v0214_part1/harness`. Every sandboxed command ran as `bash $K/sandbox.sh PROFILE [--job] -- CMD`; every gate ran through `bash -c '...'` as written in the leaf. Process counts follow the live process list, so they differ from run to run (`refused` read 940 to 969 across the runs below); the shape of each result does not. No raw process list is reproduced: only exit codes, line counts, counts and the stderr summaries. The c1810a5 and PR B cells are the values of the leaf's gates and of `__reports__/v0214_part1/` (the harness README table and the `c1810a5/` and `pr_b/` captures); they are not re-measured here.

## 1. Each profile, command by command

Cells are `<exit>`, then the stderr summary. `none` and `C` list processes, so the cell gives the stdout line count (header included); the counts differ from run to run.

| Profile | Command | c1810a5 | PR B (b716087) | PR C (a8a8644) |
|:--|:--|:--|:--|:--|
| `none` | `ps` | 0, 19 rows (harness README) | 0, 19 rows | 0, 29 lines, `hmn: 28 GPU processes found (1.3 GiB committed total).` |
| `none` | `ps --device 0` | 0 | 0 | 0, 29 lines, `hmn: 28 GPU processes found matching device=0 (1.3 GiB committed total).` |
| `none` | `ps --exit-status` | 0 | not recorded | 0, 29 lines |
| `none` | `watch 1 --duration 1s --interval 1s` | not recorded | not recorded | 0, 5 stdout lines, `hmn watch: device 0 [Apple M3 Pro], interval 1.0s, watching 1 PID(s)` |
| `C` (Codex) | `ps` | 0 | 0 | 0, 28 lines, `hmn: 27 GPU processes found (1.1 GiB committed total).` |
| `C` | `ps --device 0` | 0 | 0 | 0, 28 lines, `... matching device=0 (1.1 GiB committed total).` |
| `C` | `ps --exit-status`, `watch 1 ...` | not recorded | not recorded | 0 and 0 (`ps` 28 lines; `watch` 5 lines) |
| `P` (the report's profile) | `ps` | 0, `hmn: 0 GPU processes found.` | 2, `hmn: ps failed to query device 0: no GPU measurement source available (Metal, NVML, and nvidia-smi all failed or are disabled) (skipped)`, then `hmn: ps: no device could be queried, so nothing could be listed` | 2, no stdout, `hmn: ps failed to query device 0: process list unreadable: 954 refused, none other than the caller's could be read — re-run outside the sandbox (skipped)`, then `hmn: ps: no device could be queried, so nothing could be listed` |
| `P` | `ps --device 0` | 2, the `NoGpuSource` text naming NVML, DXGI, PDH and `nvidia-smi` | 2, the `NoGpuSource` text naming Metal, NVML and `nvidia-smi` | 2, no stdout, `hmn: ps failed to query device 0: process list unreadable: 953 refused, none other than the caller's could be read — re-run outside the sandbox` |
| `P` | `ps --exit-status` | 1 (baseline) | 2 | 2, the same two lines as `ps` |
| `P` | `watch 1 --duration 1s --interval 1s` | 2, `hmn: watch failed to query device 0: no GPU measurement source available ...` (baseline) | not recorded | 2, `hmn: watch failed to query device 0: process list unreadable: 953 refused, none other than the caller's could be read — re-run outside the sandbox` |
| `S0` (S run directly) | the same four | as `P` | as `P` | as `P`: 2, 2, 2, 2 and the same four texts (`refused` 953) |
| `L` (`(deny process-info-ledger)`) | `ps` | 0, `hmn: 0 GPU processes found.` | 0, `hmn: 0 GPU processes found.` (the residual PR C closes) | 2, no stdout, `... process list unreadable: 940 refused, none other than the caller's could be read — re-run outside the sandbox (skipped)`, then `no device could be queried` |
| `L` | `ps --device 0` | 0 | 0, `hmn: 0 GPU processes found matching device=0.` | 2, `hmn: ps failed to query device 0: process list unreadable: 940 refused, ... — re-run outside the sandbox` |
| `L` | `ps --exit-status`, `watch 1 ...` | not recorded | not recorded | 2 and 2, the `ps` and `watch` denial texts of `P` |
| `D` (`process-info-pidinfo` denied) | `ps` | 0, `(355 MiB committed total; 18 protected — re-run elevated for names)` | 0, `(927 MiB committed total; 24 protected — re-run outside the sandbox)` | 0, 29 lines, `hmn: 28 GPU processes found (1.1 GiB committed total).` (every row named, no `protected`) |
| `D` | `ps --device 0`, `ps --exit-status`, `watch 1 ...` | not recorded | not recorded | 0, 0, 0 (29, 29 and 5 stdout lines) |

Under `P` and `S0`, `hmn` reads its own 16 KiB row and drops it; under `L` its own read is denied too. Either way no other process is readable, so `others_read` is 0 and the listing is `ProcessListDenied`.

## 2. Profile S with a resident job (`--job`)

`$JOB` is the 256 MiB Metal buffer `gpujob`, started inside S's bash wrapper, a sibling of `hmn`.

| Command under `sandbox.sh S --job --` | c1810a5 | PR B (b716087) | PR C (a8a8644) |
|:--|:--|:--|:--|
| `ps` | 0, `hmn: 0 GPU processes found.` although the job is readable | `--job` is not parsed (`_: --job: command not found`, 127) | 0, 3 stdout lines (header, the `gpujob` row at `256 MiB`, `hmn`'s own row), `hmn: 2 GPU processes found (256 MiB committed total; 936 unreadable — re-run outside the sandbox).` |
| `ps --pid $JOB` | 0, `0 GPU processes found` | not run | 0, `hmn: 1 GPU process found matching pid=<JOB> (256 MiB committed total).` |
| `ps --pid $JOB --exit-status` | 1 | not run | 0 |
| `ps --device 0` | not recorded | not run | 0, `hmn: 2 GPU processes found matching device=0 (256 MiB committed total; 936 unreadable — re-run outside the sandbox).` |
| `watch $JOB --duration 1s --interval 1s` | not recorded | not run | 0, the sample row and the closing per-PID row name `gpujob` at 256 MiB, `SPILL` `n/a` |
| `watch 1 --duration 1s --interval 1s` | not recorded | not run | 0, `hmn watch: pid=1 is unreadable here; its rows will read 0 MiB — re-run outside the sandbox` |

## 3. The leaf's `[run]` gates, each as written

All six built with `cargo build --release --locked`.

| Gate | Result | Before |
|:--|:--|:--|
| the profile run (`P`, `S0`, `L` × `ps`, `ps --device 0`) | `P 2`, `P device 2`, `S0 2`, `S0 device 2`, `L 2`, `L device 2`, then `denial text in 6 of 6 files, remedy in 6` | b716087: `P 2`, `P device 2`, `S0 2`, `S0 device 2`, `L 0`, `L device 0`, 0 of 6 files, remedy in 0 |
| `S --job`: the same-sandbox row | `exit 0` then `1`; stderr `hmn: 2 GPU processes found (256 MiB committed total; 948 unreadable — re-run outside the sandbox).` | b716087: no `exit` line, `grep` exit 2 (`--job` absent) |
| `D`: names, not `?` | `exit 0`: every row named, no `protected` and no `re-run elevated` in stderr; stderr `hmn: 28 GPU processes found (802 MiB committed total).` | b716087: `exit 1`, 25 rows, 24 nameless, `24 protected — re-run outside the sandbox` |
| the seam check (profile S with the job; `hmn ps` and `count_denied.py` in one sandbox run) | `hmn=947 probe=947`, `rc=0` (the probe's own line: `denied=947 read=3 gone=0`) | b716087: `hmn= probe=`, exit 1 |
| `compare.py final_base final` | `rc=0`, `identical after normalisation` twice, `rows compared: 55` and `55`, `spill cells mapped: 0` twice | b716087: the directory did not exist, `rc=2` |
| `ps --device 1` and `watch 0 --duration 2s --interval 1s` | `1` (`hmn: ps failed to query device 1: device index 1 out of range (have 1 devices)`, exit 2), then `0` (no `names no running process` line) | b716087: `1` and `0` |

## 4. The captures and `--help`

`captures/final_base/` is b716087's release build and `captures/final/` is this tree's, both from `capture.py` in the same boot, base first, then this tree, with the raw directories under `/private/tmp`. `compare.py final_base final` (policies `none` and `C`; no `--spill-map`): `none: identical after normalisation (rows compared: 55, spill cells mapped: 0, allowlisted lines: 0)` and the same line for `C`, exit 0. Where libproc answers, PR C's output equals PR B's. The capture files also hold the policy `P`, where the two builds differ as sections 1 and 3 state; `compare.py` does not compare it.

The `--help` text is the one intended difference. `hmn --help`, `hmn ps --help` and `hmn watch --help` of b716087 against this tree, line by line, each changed line traced to the leaf that owns it:

| Output | Changed line | Leaf |
|:--|:--|:--|
| `hmn --help`, the `ps` summary | adds `; the processes it cannot read are counted on the summary line as unreadable` | ps_watch_unreadable |
| `hmn --help`, Security note | `On macOS a bare ?` now reads `means both name lookups failed or the process is gone` | part2_close, Step 1 |
| `hmn --help`, the macOS bullet | adds `hmn counts the processes it cannot read and exits 2 under ps --exit-status when it cannot tell` | ps_watch_unreadable |
| `hmn ps --help`, the description | the same addition as the `hmn --help` summary | ps_watch_unreadable |
| `hmn ps --help`, `--pid` | adds `An unreadable process counts as a possible match, so --pid N reports only N among the unreadable ones` | ps_watch_unreadable |
| `hmn ps --help`, `--filter` | adds `On macOS a name read from the kernel's p_comm is cut at 16 bytes, and a pattern cannot match past the cut` | kinfo_enumeration |
| `hmn ps --help`, `--exit-status` | adds `or left unreadable a process the filters could match`, and `no process the filters could match was unreadable` | ps_watch_unreadable |
| `hmn watch --help`, the description | adds the unreadable-PID attach warning and `--follow-new says how many processes it cannot follow` | ps_watch_unreadable |
| `hmn watch --help`, `--filter` | the same `p_comm` sentence as `hmn ps --help` | kinfo_enumeration |

## 5. The App Sandbox, no profile

Built as the recipe of `__reports__/field_check_v0213/evidence/probes/appsandbox/README.md` says: `RUSTFLAGS="-C link-arg=-Wl,-sectcreate,__TEXT,__info_plist,<abs path>/Info.plist" cargo build --release --locked --target-dir target/appsb`, then `codesign -s - -f --entitlements <abs path>/entitlements.plist target/appsb/release/hmn`. `codesign -d --entitlements -` shows `com.apple.security.app-sandbox` `[Bool] true`. The signed binary was copied to `mktemp -d /private/tmp/hmn-appsb.XXXXXX` and run from there (in place under `~/Documents` it dies with `SIGTRAP`, exit 133).

The leaf's gate prints:

```
1
[ps] 2 unreadable=1 skipped=1
[ps --device 0] 2 unreadable=1 skipped=0
[watch 1 --duration 1s --interval 1s] 2 unreadable=1 skipped=0
```

and the stderr of the three runs is `hmn: ps failed to query device 0: process list unreadable: 969 refused, none other than the caller's could be read — re-run outside the sandbox` (with ` (skipped)` and `hmn: ps: no device could be queried, so nothing could be listed` for plain `ps`; `hmn: watch failed to query device 0: ...` for `watch`). Before: b716087 `[ps] 2 unreadable=0 skipped=1`, `[ps --device 0] 2 unreadable=0 skipped=0`, `[watch ...] 2 unreadable=0 skipped=0` (the `NoGpuSource` text); 40a701e `[ps] 0`, `hmn: 0 GPU processes found.`

## 6. Claude Code's sandbox

`claude_code_sandbox/pr_c.md` is the evidence file of a Bash call the user ran inside Claude Code 2.1.273 with its sandbox enabled (`/sandbox`, `user-confirmed: yes`): `nested sandbox-exec rc=71`, `envsandbox: SANDBOX_RUNTIME=1`, `sandboxed: 1`, hmn commit c026a93. `hmn ps` listed 28 GPU processes with no unreadable count, `hmn ps --device 0` the same, and `hmn watch 1` ran its duration and exited 0. `outcome: full`: the output equals the unsandboxed output, so no profile that denies `process-info` applies there. The file is copied unchanged; the run was not repeated here.

## 7. The rows with no profile, and the CI branch

- `cargo test --locked --all-features --test macos_smoke -- --ignored`: 4 passed (`-- --list` counts 4: `device_info_reports_apple_brand`, `process_exists_under_sandbox_profiles`, `process_gpu_info_returns_metal_source`, `gpu_process_listing_under_sandbox_profiles`). Before: 3 at b716087.
- `cargo test --locked --all-features --target x86_64-apple-darwin --lib` (Rosetta): 118 passed, 0 failed; the same natively. Before: 95 at b716087.
- The 648-byte `kinfo_proc` layout holds on arm64 natively and on x86_64 under Rosetta (the lib tests above); real Intel hardware is untested.
- `ci_macos_cli_ps.md`: run 37650659228, head c026a93, all 8 jobs success; on both macos-latest runners `branch=expected` fired for `with --exit-status` and `without --exit-status`, so neither `skipped-device` nor `skipped-device-nogpu` fired.

## 8. Defects

None: every row came out as its gate says.
