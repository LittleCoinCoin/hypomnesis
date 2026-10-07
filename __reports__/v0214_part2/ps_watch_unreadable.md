# ps_watch_unreadable: what `hmn ps` and `hmn watch` say under each sandbox profile

`hmn ps` and `hmn watch` list through `gpu_process_listing`. They count the processes the caller was refused (`N unreadable — re-run outside the sandbox`), say so once at attach in `hmn watch`, and exit `2` where "nothing listed" would otherwise read as "nothing matched". This record is the measured before and after of what the leaf can judge from the command line, run through the one harness in `__reports__/v0214_part1/harness/`.

Host: Apple M3 Pro, macOS 26.6.2, arm64, stable Rust 1.99.0 (`rustup check`: up to date). Binaries, all `cargo build --release --locked` (default features, so no `[nvidia-smi debug]` line):

| Column | Tree |
|:--|:--|
| b716087 | PR B merged, built in a detached worktree under `target/base` |
| after gpu_process_listing | this leaf's base: b716087 plus `kinfo_enumeration` and `gpu_process_listing`, built in a detached worktree under `target/base2` |
| after | the tip of `task/ps_watch_unreadable` |

Both detached worktrees were removed afterwards. `H` is the `hmn` binary of the tree, `K` is `__reports__/v0214_part1/harness`. Every command ran as `bash $K/sandbox.sh PROFILE [--job] -- CMD`, from the repository root, under `bash -c`. The b716087 column ran the b716087 binary through the harness of this tree, whose `--job` b716087 itself lacks. Counts of processes follow the live process list, so they differ from run to run (1066 to 1145 across these runs); the shape of each result does not. No process list is reproduced here: only counts, exit codes and the stderr lines.

## `hmn ps`

Each cell is the exit code and the stderr line (counts as measured in that run). `JOB` is the 256 MiB `gpujob` started inside S's bash wrapper, a sibling of `hmn`; `WS` is `WindowServer`.

| Command | b716087 | after gpu_process_listing | after |
|:--|:--|:--|:--|
| S: `ps` | exit 2, the `NoGpuSource` skip line, then `hmn: ps: no device could be queried, so nothing could be listed` | exit 0, `hmn: 1 GPU process found (0 MiB committed total).` | exit 0, `hmn: 1 GPU process found (0 MiB committed total; 1082 unreadable — re-run outside the sandbox).` |
| S: `ps --pid WS --exit-status` | exit 2, the two `NoGpuSource` lines | exit 1, `hmn: 0 GPU processes found matching pid=<WS>.` | exit 2, `hmn: 0 GPU processes found matching pid=<WS> (1 unreadable — re-run outside the sandbox).` |
| S: `ps --pid 4294967295 --exit-status` | exit 2, the two `NoGpuSource` lines | exit 1, `hmn: 0 GPU processes found matching pid=4294967295.` | exit 1, the same line |
| S `--job`: `ps --json --pid JOB` | exit 2, empty stdout | exit 0, one object, nine keys | exit 0, one object, the same nine keys (`pid,name,used_bytes,shared_used_bytes,device_index,device_name,spilling,paged,shared_share`), no `unreadable` in the JSON; stderr `hmn: 1 GPU process found matching pid=<JOB> (256 MiB committed total).` |
| P: `ps` | exit 2, the `NoGpuSource` skip line | exit 2, `hmn: ps failed to query device 0: process list unreadable: 1082 refused, none other than the caller's could be read (skipped)`, then the closing line | exit 2, the same line with ` — re-run outside the sandbox` before ` (skipped)`, then the closing line |
| P: `ps --device 0` | exit 2, the `NoGpuSource` line | exit 2, the denial line without remedy | exit 2, `hmn: ps failed to query device 0: process list unreadable: 1082 refused, none other than the caller's could be read — re-run outside the sandbox` |
| S0: `ps`, `ps --device 0` | as P | as P | as P |
| L: `ps` | exit 0, `hmn: 0 GPU processes found.` | exit 2, the denial line, `(skipped)` | exit 2, the denial line with the remedy, `(skipped)` |
| L: `ps --device 0` | exit 0, `hmn: 0 GPU processes found matching device=0.` | exit 2, the denial line | exit 2, the denial line with the remedy |

Under S the table and `--json` hold `hmn`'s own 16 KiB row (`0 MiB`), because the caller reads itself; `--pid` drops it. WindowServer is what the "after gpu_process_listing" column gets wrong: exit `1`, "nothing matched", for a process that exists and was refused.

## `hmn watch`

| Command (S unless noted) | b716087 | after gpu_process_listing | after |
|:--|:--|:--|:--|
| `--job`: `watch 1 JOB --duration 2s --interval 1s` | exit 2, the `NoGpuSource` text | exit 0, `gpujob` at 256 MiB in 3 rows; no notice | exit 0, 3 `gpujob` rows at 256 MiB, and once at attach `hmn watch: pid=1 is unreadable here; its rows will read 0 MiB — re-run outside the sandbox`; no `names no running process` |
| P: `watch 1 --duration 2s --interval 1s` | exit 2, the `NoGpuSource` text | exit 2, `hmn: watch failed to query device 0: process list unreadable: 1082 refused, none other than the caller's could be read` | exit 2, the same line ending ` — re-run outside the sandbox` |
| `watch --duration 2s --interval 1s` | exit 2, the `NoGpuSource` text | exit 0, watching 1 PID(s) (`hmn` itself) | exit 0, watching 1 PID(s), and once `hmn watch: device 0: 1082 unreadable — re-run outside the sandbox; they are not followed`; no `found no GPU processes` |
| `watch --min 1MiB --duration 2s --interval 1s` | exit 2, the `NoGpuSource` text | exit 2, `hmn: watch found no GPU processes on device 0 to auto-select (top 5 with footprint >= 1 MiB); re-run with an explicit PID once a workload is running` | exit 2, the same line with `; 1082 unreadable — re-run outside the sandbox` after the criterion, inside the parentheses |
| `watch --follow-new --duration 2s --interval 1s` | exit 2, the `NoGpuSource` text | exit 0, following top 5, 1 initially | exit 0, the same header, then the `they are not followed` line once |
| `watch --min 1MiB --follow-new --duration 2s --interval 1s` | exit 2, the `NoGpuSource` text | exit 0, `found no GPU processes on device 0 yet (top 5 by committed with footprint >= 1 MiB); waiting for work to appear` | exit 0, the same line with `; 1066 unreadable — re-run outside the sandbox` inside the parentheses; no `they are not followed` line |

The count is said once per attach: the `found no GPU processes` line carries it when nothing was selected, and the `they are not followed` line carries it when something was. The per-interval `sample failed` line holds no remedy.

## The leaf's `[run]` gates, as run on the tip

Each line is what the gate's command printed. `bash -c`, `H="$PWD/target/release/hmn"`.

| Gate | Printed | Verdict |
|:--|:--|:--|
| profile S count against the independent probe (the count is read only from `N unreadable — re-run outside the sandbox)`) | `hmn=1134 probe=1136`, difference 2 (at most 5), exit 0; stderr `hmn: 1 GPU process found (0 MiB committed total; 1134 unreadable — re-run outside the sandbox).` | pass |
| S `--pid` WindowServer and 4294967295 (`unreadable` counted only beside its remedy) | `pid=391 exit 2 unreadable=1` with `hmn: 0 GPU processes found matching pid=391 (1 unreadable — re-run outside the sandbox).`; `pid=4294967295 exit 1 unreadable=0` | pass |
| S `--job` JSON | `exit 0`; stderr `hmn: 1 GPU process found matching pid=<JOB> (256 MiB committed total).`; the nine keys; `0` | pass |
| P and S0, `ps` and `ps --device 0` | `P 2 1`, `P device 2 1`, `S0 2 1`, `S0 device 2 1` | pass |
| L, `ps` and `ps --device 0` | `[ps] 2 1`, `[ps --device 0] 2 1` | pass |
| `explained_by_denial_needs_exit_2_and_the_denial_on_one_skip_line` | 1 listed, 1 passed; the ignored test listed once, `--include-ignored` passes; `explained_by_denial` counts `12`, in `fn branch` `1`, `branch(` in `fn accept` `1`, `skipped-device-nogpu` `4` | pass |
| `cargo test --test cli_ps` under P (guard) | `exit 0`, `2` lines `branch=skipped-device`, `0` lines `branch=skipped-device-nogpu` | pass |
| S `--job` watch (adds the `they are not followed` count) | `exit 0`, `1`, `0`, `0`, `3` | pass |
| watch under P and S | `P exit 2 1`, `S auto exit 0 notfollowed=1 nofound=0`, `S min exit 2 1`, `S follow exit 0 unreadable=1`, `S min follow exit 0 unreadable=1 notfollowed=0` | pass |

The `cli_ps` branch lines under P are the denial lines now, and `accept` labels them `skipped-device` through `explained_by_denial`. A run whose CLI stayed on the `NoGpuSource` skip line would move the two counts to `0` and `2`. Which label fires on each `macos-latest` job is part2_close's to record from the PR's CI: R03 expects a VM whose ledger template does not resolve to give the `NoGpuSource` line, which `accept` takes under `skipped-device-nogpu`.

## Smoke checks: unsandboxed and Codex profile (C)

| Profile | `ps` | `ps --json` | `watch` | `unreadable` on stderr | `re-run elevated` on stderr |
|:--|:--|:--|:--|:--|:--|
| none | exit 0, `hmn: N GPU processes found (M MiB committed total).` | exit 0 | `watch $JOB --duration 2s --interval 1s` exit 0, 3 `gpujob` rows | 0 | 0 |
| C | exit 0, the same shape | exit 0 | `watch --duration 2s --interval 1s` exit 0, watching 5 PID(s) | 0 | 0 |

`watch $JOB` cannot run under C: `sandbox.sh C --job` exits `70` (`gpujob: no Metal device`), because Codex's policy denies the Metal device to the job. Under C the watch was run by auto-selection instead, and `watch 1 --duration 2s --interval 1s` (an explicit PID) exits 0 and prints no notice. part2_close owns the byte comparison against the part1_close baselines through `compare.py`.

## What pins the Windows and Linux output

`ProcessListDenied` and `denied_pids` exist on every platform, but only macOS fills them: `listing_without_denials_has_no_denied_pids` (a lib test that runs on every OS, in the `gpu_process_listing` leaf) pins `denied_pids` empty for the NVML, PDH and `nvidia-smi` arms. With nothing refused, this leaf's code paths reduce to the text that existed before:

- the six `format_ps_summary_*` tests that pin `N protected — re-run elevated for names` call `format_ps_summary_with(.., false)` and run on every OS, unchanged; `git diff b716087 -- src/bin/hmn/ps.rs` removes no such literal;
- `format_ps_summary_unreadable_zero_changes_nothing` pins the macOS and the elevated text for a zero count;
- `ps_exit_code_table_pins_the_exit_status_rule` covers all 16 input cells, PR B's eight rows among them with `relevant_denied` 0;
- `unresolved_growth_hint_keeps_the_elevated_text_off_macos` is untouched;
- with an empty denied list `denied_pid_notices_name_each_denied_explicit_pid_in_the_order_given` gets no notice, and `missing_pid_notices_name_only_pids_known_not_to_exist` is unchanged; the "found no GPU processes" lines keep their text when the count is 0, which is inline code in `run_watch`.

None of these is a test that passes vacuously on a runner with no GPU: each takes `outside_sandbox` or the denied list as a literal.

## Residuals

Wiring that no cargo test reaches, found by mutation and stated here and in the PR body:

- `run_watch` passing `&[]` to `missing_pid_notices` is caught by nothing: PR B's `process_exists` answers `Some(true)` for a denied PID, so `names no running process` cannot appear for it. The unit test `missing_pid_notices_skip_denied_pids` pins the function.
- The unfiltered `denied_pids.len()` at the `run_ps` accumulation, and `&[]` passed to `denied_pid_notices` in `run_watch`, are caught only by the CLI gates above (`--pid 4294967295` exits 1, and the `pid=1 is unreadable here` notice count).
- The "found no GPU processes" count, the `--follow-new` count line and the per-interval `sample failed` line are inline in `run_watch` and pinned by the profile-S runs above, not by a cargo test.
- A denied list with a duplicate PID would count it twice. The enumerations never list a PID twice.
