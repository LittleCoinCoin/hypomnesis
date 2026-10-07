# Behavioural equivalence: A (v0214-part2 @ 03bab46) vs B (v0214-part2-slim @ c070d49)

## Verdict: EQUIVALENT

The only behavioural differences are the intended ones: the c070d49 `watch` notice change and the
matching `watch --help` sentence. A against B0 (80c57c3) is identical after normalisation in all
528 cells. B0 against B differs in exactly 11 cells, all intended. Both edge changes the slimmers
named are invisible on this host. The slimming did remove the failing test for several agreed
behaviours that A pinned (see Coverage): B has no test for the watch notice, `accept`'s labels,
the retry bound, "libproc first" or the 4G saturation, nor for `GpuProcessListing`'s
`#[non_exhaustive]`.

Everything ran in one boot (kern.boottime Tue Oct 6 22:03:16 2026), M3 Pro, macOS 26.x, WindowServer
pid 391, default-feature release builds (`cargo build --release --locked`) of eq_A, eq_B0, eq_B.
All scratch worktrees (eq_A, eq_B, eq_B0, eq_G, eq_AM, eq_BM) were removed afterwards; no existing
worktree was touched. Scripts and raw output stay in the scratchpad: `eq_run.sh`, `eq_cmp.py`,
`eq_mut.py`, `eq_gates.sh`, `eq_out/`, `eq_gate_logs/`, `eq_mut_*.json`, `eq_cap_*`.

## 1. Builds

| tree | commit | `cargo build --release --locked` |
|:--|:--|:--|
| eq_A | 03bab46 | ok |
| eq_B0 | 80c57c3 | ok |
| eq_B | c070d49 | ok |

## 2. CLI equivalence

Method: `eq_run.sh` ran every command with A, B0 and B back to back, through `bash -c` and
`sandbox.sh PROFILE -- hmn ...`, under the profiles none, P, S, S0, Q, D, L, C (18 commands each)
plus `--job` variants (`sandbox.sh PROFILE --job -- bash -c 'hmn ... $JOB'`) under every one of
those profiles, including S `--job`: `watch 1 $JOB`, `ps --json --pid $JOB`, `ps --pid $JOB
--exit-status`, `watch --duration 2s --interval 1s`. That is 22 slugs x 8 profiles x 3 binaries =
528 recorded runs, each with stdout, stderr and rc. The 18 plain commands are: `ps`, `ps --json`,
`ps --device 0`, `ps --device 1`, `ps --pid 391 --exit-status`, `ps --pid 4294967295
--exit-status`, `ps --filter zzz --exit-status`, `ps --min 1GiB --exit-status`, `watch 0`,
`watch 1`, `watch`, `watch --min 1MiB`, `watch --follow-new`, `watch --min 1MiB --follow-new`
(each `--duration 2s --interval 1s`), `--help`, `ps --help`, `watch --help`, and `--version`.
(`watch 0` and `watch 1` are PIDs 0 and 1, as in the brief.)

Normalisation (`eq_cmp.py`): sizes (including `+0 B` and `-0 MiB`), `+N.Ns` times, PIDs (table
rows, `pid=N`, `PID N`), `N GPU process(es) found`, `N unreadable`, `N refused`, `N protected`,
`N unnamed`, `watching N PID(s)`, `N initially`. Table and watch rows are compared as the set of
distinct normalised lines, so row order and process count drift are tolerated but names, devices
and SPILL cells are kept. `--json` is compared as the set of per-row (key, type) signatures. rc and
the rest of stderr are compared exactly.

Result: A vs B0 = 0 differences in 528 cells. B0 vs B = 11 differences in 528 cells.

### Difference table (every difference found)

| # | command | profile | A vs B0 | B0 vs B | class |
|:--|:--|:--|:--|:--|:--|
| 1 | `watch --help` | none, P, S, S0, Q, D, L, C (8 cells) | identical | stdout differs: "Auto-selection and `--follow-new` say how many processes they cannot follow." becomes "`--follow-new` says how many processes it cannot follow." (word diff: Auto-selection, and, say to says, they to it) | intended (c070d49, help sentence) |
| 2 | `watch --duration 2s --interval 1s` | S | identical | stderr: B0 prints `hmn watch: device 0: N unreadable — re-run outside the sandbox; they are not followed`, B does not | intended |
| 3 | `watch --duration 2s --interval 1s` (from `--job`) | S `--job` | identical | same line gone | intended |
| 4 | `watch --duration 2s --interval 1s` (from `--job`) | S0 `--job` | identical | same line gone | intended |

Not different, and checked: in S, `watch --follow-new` prints the line in A, B0 and B (count 1 each).
No other command or profile prints the line (it needs a readable row plus an unreadable PID).
`hmn --help`, `hmn ps --help` and `--version` are byte-identical across A, B0 and B. `watch --help`
is byte-identical A to B0 in all 8 profiles.

Output facts that held in all three (spot-check of the B column): none, D and C list ~23 rows,
exit 0; P, S0 and L give `process list unreadable: N refused, none other than the caller's could be
read — re-run outside the sandbox (skipped)` and exit 2; Q gives `no GPU measurement source
available (Metal, NVML, and nvidia-smi all failed or are disabled)` and exit 2; S lists 1 row with
`N unreadable — re-run outside the sandbox`, `--pid 391 --exit-status` exits 2 and `--pid
4294967295 --exit-status` exits 1; `ps --device 1` exits 2 (out of range) everywhere.

### Harness
`python3 __reports__/v0214_part1/harness/capture.py` for A, B0, B (raw dirs outside the repos), then
`compare.py`:

- A vs B0: `none: identical (rows compared 45, spill cells mapped 0, allowlisted 0)`, `C: identical (45, 0, 0)`, rc 0.
- B0 vs B: same, both identical, rc 0. (A vs B: same.)
- The P captures (not compared by `compare.py`): A vs B0 byte-identical; B0 vs B differ only in the live
  count (`1091 refused` vs `1096 refused`), which is process churn between runs.
- Independent check: under P, `hmn ps` reports `1092 refused` for A, B0 and B on two consecutive rounds;
  `count_denied.py` under P gives `denied=1091` (it also excludes its own PID).

### Extra probe (not in the brief): profile X, the one place A and B read a name differently
In B the `p_comm` name always comes from the per-PID `KERN_PROC_PID` lookup; in A, after a refused
`proc_listpids`, it came from the `KERN_PROC_ALL` record. A profile that reaches that path:
`(deny process-info-listpids)(deny process-info-pidinfo)(allow process-info-pidinfo (target self))`
(libproc list refused, `proc_pidpath` refused, ledger allowed). `hmn ps` under it: A, B0 and B all
print 23 rows, rc 0, names cut at 16 bytes (`com.apple.WebKit`), so `p_comm` is the source in all
three; the per-column diff of A against B0 and B is empty. Invisible.

## 3. Library equivalence

- No nightly installed. I produced rustdoc JSON on stable with `RUSTC_BOOTSTRAP=1 cargo rustdoc
  --lib --all-features -- -Z unstable-options --output-format json` (format 61) for A and B and
  compared the crate-local items (486 each, ids mapped to names, `docs`, spans and links removed):
  identical, including every public signature of `gpu_process_listing`, `gpu_processes`,
  `GpuProcessListing` and `HypomnesisError`, `#[non_exhaustive]` and the other attrs (attrs differ
  only in source line numbers inside cfg traces).
- `src/lib.rs`: `git diff 03bab46 c070d49 -- src/lib.rs` is empty, so the `pub use` lists are identical.
- Rustdoc prose differs (docs only) for three items: `GpuProcessListing`, `HypomnesisError`,
  `gpu_process_listing`. Notable: B drops the `HypomnesisError` paragraph saying `ProcessListDenied`'s
  `Display` states the count and carries no remedy (the unit test of that name remains; no rustdoc in B
  says it now), and B drops `GpuProcessListing`'s two doctests (see Coverage, N1). The `gpu_process_listing`
  platform table is shorter but states the same rules.
- `ProcessListDenied`'s `Display`: the `#[error("process list unreadable: {denied} refused, none other
  than the caller's could be read")]` string is unchanged (the `src/error.rs` diff is the doc paragraph
  only, 4 lines). It prints identically in the CLI matrix under P, S0 and L.
- `#[ignore]`d sandbox listing tests, run on both (all pass):
  - A `cargo test --test macos_sandbox -- --ignored --nocapture` prints, per profile: S `LISTING ok
    denied=1086 entries=0 caller_denied=false overlap=false`; S0, P, L `LISTING err=ProcessListDenied
    denied=1086`; unsandboxed `LISTING ok denied=0 entries=23 ...`.
  - B `cargo test --test macos_smoke -- --ignored gpu_process_listing_under_sandbox_profiles` passes
    (children print one label each). To compare what each child reports, I ran both test binaries'
    child role directly under each profile (`HMN_GPL_CHILD=1 sandbox-exec ... --exact
    gpu_process_listing_under_sandbox_profiles --nocapture`):

| profile | A child | B child |
|:--|:--|:--|
| P | `err=ProcessListDenied denied=1086` | `denied` |
| S0 | `err=ProcessListDenied denied=1086` | `denied` |
| S (resident bash) | `ok denied=1086 entries=0 caller_denied=false overlap=false` | `partial` |
| L | `err=ProcessListDenied denied=1086` | `denied` |
| Q | `err=NoGpuSource denied=0` | `NoGpuSource` |
| D | `ok denied=0 entries=23 ...` | `open` |
| C | `ok denied=0 entries=23 ...` | `open` |

  Same answers. B's test exercises S, S0, L and unsandboxed (P was dropped; it asserts the label only,
  not the `denied >= 1` count or the entry/denied `overlap` field that A checked).

## 4. Gates on B, each commit on its own (eq_G, checked out per commit)

Every cell is exit status 0 with 0 `warning` lines in the log.

| gate | 448b5f0 | f0a278b | 80c57c3 | c070d49 |
|:--|:--|:--|:--|:--|
| `cargo fmt --check` | 0 | 0 | 0 | 0 |
| clippy `-D warnings`, default | 0 | 0 | 0 | 0 |
| clippy `--all-features` | 0 | 0 | 0 | 0 |
| clippy `--all-features --target x86_64-unknown-linux-gnu` | 0 | 0 | 0 | 0 |
| `cargo test --locked --all-features` | 0 (434 passed, 14 ignored) | 0 (426, 14) | 0 (396, 14) | 0 (396, 14) |
| `cargo doc` `-D warnings` `--all-features --no-deps` | 0 | 0 | 0 | 0 |
| `+1.88` check `--all-features` | 0 | 0 | 0 | 0 |
| `+1.88` clippy `--all-targets --all-features` | 0 | 0 | 0 | 0 |
| `+1.88` test `--all-features` | 0 (434) | 0 (426) | 0 (396) | 0 (396) |
| `check --no-default-features` | 0 | 0 | 0 | 0 |
| `check --no-default-features --features nvml,dxgi,pdh` | 0 | 0 | 0 | 0 |
| `test --no-default-features --lib --no-run` | 0 | 0 | 0 | 0 |
| Rosetta `--target x86_64-apple-darwin --lib` | 0 (123 passed) | 0 (118) | 0 (118) | 0 (118) |
| `--ignored` `macos_smoke` | 0 (3 passed) | 0 (4) | 0 (4) | 0 (4) |
| `--ignored` `macos_sandbox` (exists only at 448b5f0) | 0 (1) | n/a | n/a | n/a |
| `--ignored` `cli_ps` | 0 (1) | 0 (1) | 0 (1) | 0 (1) |

A for reference (all-features): lib 146, hmn bin 280, cli_ps 8, macos_smoke 6, smoke 10, doctests 6.
B at c070d49: lib 118, bin 256, cli_ps 2, macos_smoke 5, smoke 10, doctests 5. `cargo deny` and
`cargo package --list` were not in this task's gate list and were not run (`cargo deny` is not installed).

## 5. Coverage

Method: mutate the behaviour once in a copy of B (eq_BM at c070d49) and run `cargo test --locked
--all-features`, then, if that passed, `--ignored` for `macos_smoke` and `cli_ps`. The same mutation
(adapted to A's code shape) was run on a copy of A (eq_AM at 03bab46, `--ignored` adds `macos_sandbox`).
CAUGHT = a test failed. SURVIVED = nothing failed. Mutation ids are in `eq_mut.py`.

| behaviour | test in B that pins it | B mutation: result | A had a failing test? |
|:--|:--|:--|:--|
| EPERM-only denial | `footprint_from_errno_only_eperm_is_denied`, `libproc_outcome_only_eperm_is_refused` | 1a any errno denied, 1b any errno refuses libproc, 1c `EPERM` = 13: all CAUGHT | yes (split tests) |
| `p_comm` only on EPERM | `name_after_pidpath_uses_comm_only_when_eperm` | 2a comm after an unclassified failure, 2b comm for ESRCH, 2c comm preferred over a path: CAUGHT | yes |
| `p_comm` wiring: a refused-pidpath row is named from `p_comm` | none | 2d (the lookup returns `None`): SURVIVED | no, SURVIVED in A too |
| libproc first, kinfo only when libproc says EPERM | none | 3a kinfo also run when libproc answered, 3b kinfo fallback when libproc merely failed, 3c kinfo tried first: all SURVIVED | yes: `enumerate_pids_with_keeps_libproc_pids_and_never_runs_kinfo`, `..._falls_back_only_when_libproc_says_eperm` caught 3a/3b/3c |
| `ENOMEM` partial fill never parsed | `classify_kinfo_all_enomem_is_retry_even_when_the_buffer_holds_records` | 4a ENOMEM not a retry, 4b ENOMEM falls through to parsing, 4c len read before rc: CAUGHT | yes |
| the retry bound (4 attempts) | none (the loop is inline in `list_kinfo_all`) | 5a bound 1, 5b bound 9: SURVIVED | yes: `fill_kinfo_all_with_gives_up_after_exactly_the_attempt_limit` (+1 more) |
| the caller never denied | `tally_reads_does_not_count_the_caller_as_denied_or_as_another_process` | 6a caller counted denied, 6b caller counted as another process read: CAUGHT | yes |
| PID <= 0 skipped (PID 0 never listed or counted) | `tally_reads_counts_every_read_pid_and_lists_a_row_for_a_non_zero_balance` | 6c `pid < 0`: CAUGHT | yes |
| `ProcessListDenied` only when nothing else was read | `decide_listing_is_denied_when_only_the_callers_own_row_was_read`, `decide_listing_with_nothing_denied_is_ok_even_when_empty`, `decide_listing_sorts_by_pid_and_keeps_denied_pids` | 7a denied whenever anything denied, 7b whenever nothing else read, 7c `others_read <= 1`: CAUGHT; 7e unsorted rows: CAUGHT | yes |
| the denied count saturates at `u32::MAX` | none (needs a 2^32-element list) | 7d `unwrap_or(0)`: SURVIVED | yes: `process_list_denied_saturates_the_count` |
| `ProcessListDenied` `Display` | `error::tests::process_list_denied_display_states_the_count_and_no_remedy` | 7f extra `.`: CAUGHT | yes |
| `denied_pids` empty off macOS | `listing_without_denials_has_no_denied_pids` (compiled only for nvml-on-Linux, pdh-on-Windows, nvidia-smi-fallback; runs here under `--all-features`) | 8a carries PID 1: CAUGHT | yes (same test) |
| `unreadable` clause wording and one remedy (ps) | `format_ps_summary_unreadable_counts_with_the_macos_remedy`, `format_ps_summary_unreadable_and_protected_say_the_remedy_once` and the other `format_ps_summary_*` | 9a `refused`, 9b remedy twice, 9c wrong remedy flag: CAUGHT | yes (more tests, incl. `remedy_clause_*`) |
| same clause on the `watch` call site | none | 9d `remedy_clause(0, n, false)`: SURVIVED | no, SURVIVED in A too |
| watch "found no GPU processes ... to auto-select" line | none (inline `eprintln!`) | 9e reworded: SURVIVED | yes: `no_processes_line_*` caught 9e |
| `--pid` narrowing | `relevant_denied_counts_every_denied_pid_or_only_the_pid_asked_for`, `relevant_denied_ignores_min_and_filter` | 10a ignores `--pid`: CAUGHT | yes |
| `--pid` narrowing, wiring in `run_ps` | none | 10b `+= denied_pids.len()`: SURVIVED | no, SURVIVED in A too |
| `--exit-status` 2 | `ps_exit_code_table_pins_the_exit_status_rule` | 11a drop relevant-denied, 11b drop failed device, 11d also with rows: CAUGHT | yes |
| `--exit-status` 2, wiring in `run_ps` | none (the `--ignored` `cli_ps` test covers the failed-device path only) | 11c passes 0 for the unreadable count: SURVIVED | no, SURVIVED in A too |
| the watch notice (`--follow-new` only, c070d49's intended change) | none | 12a notice for explicit PIDs too, 12b never printed, 12c plain auto-select only (B0's behaviour), 12d reworded, 12e: all SURVIVED | partly: `not_followed_notice_*` caught wording (12d); its call-site gating (12a, 12b, 12e) SURVIVED in A |
| `accept`'s labels and its refusal of a bare exit 2 | none | 13a bare exit 2 accepted, 13b and 13c label renamed, 13d NoGpuSource line at any exit code, 13e any line for the denial branch: SURVIVED | yes: `accept_fails_on_a_bare_exit_2`, `branch_labels_each_accepted_case_and_rejects_the_rest` caught 13a-13d |
| `failure_detail`: remedy on a denial only | `failure_detail_appends_the_remedy_to_a_denial_only` | 14a never appends: CAUGHT; 14b also appends to `NoGpuSource` (the macOS VM case): SURVIVED | yes: `failure_detail_leaves_other_errors_alone` caught 14b |
| libproc success with an empty list is a failure; record-size guard | `libproc_outcome_an_empty_list_is_failed`; `trust_kinfo_listing_keeps_records_only_when_self_is_one_648_byte_record` | 15a, 15b, 15c: CAUGHT | yes |
| N1: `GpuProcessListing` stays `#[non_exhaustive]` (item 4's API contract) | none | removing `#[non_exhaustive]`: SURVIVED (all tests pass) | yes: the `compile_fail` doctest fails (`snapshot::GpuProcessListing (line 235) - compile fail ... FAILED`) |

Behaviours with no failing test in B: libproc-first and the kinfo-only-on-EPERM rule (3a-3c), the
retry bound (5a, 5b), count saturation (7d), the watch notice including c070d49's `--follow-new`
gate (12a-12e), the watch "found no GPU processes" wording (9e), `accept`'s labels and
bare-exit-2 refusal (13a-13e), the `NoGpuSource`-gets-no-remedy rule (14b), `#[non_exhaustive]`
(N1). A had a failing test for every one of those except the call-site wiring parts of the watch
notice (12a, 12b, 12e). Both A and B leave the call-site wiring unpinned for 2d, 9d, 10b, 11c.
3a, 3c, 5a, 5b and 7d change no output on this host (3c returns the same PIDs from either source;
3b differs only when libproc fails for a reason other than EPERM, which cannot be provoked here), so no
live test could catch them.
12a-12c and 13a are the ones that change user-visible output or test strictness.

## 6. The slimmers' edge changes

1. Ledger read now runs for PID 0 before the tally skips it. Invisible.
   - Mechanism: A filtered `pid > 0` before `read_graphics_footprint`; B passes every PID and
     `tally_reads` skips `pid <= 0` before it looks at the read. The read for PID 0 is one extra
     `ledger` syscall. Probed: unsandboxed it succeeds (errno 0); under P it is `EPERM`; the skip
     discards both, so `denied_pids`, `others_read` and `entries` are unchanged.
   - Evidence: A vs B0 identical in all profiles incl. P, L, D, C; the refused count is `1092` for A,
     B0 and B twice in a row under P; mutation 6c (not skipping PID 0) is caught by
     `tally_reads_counts_every_read_pid_and_lists_a_row_for_a_non_zero_balance`. Residual: one more
     sandbox denial record in the system log under P, S and L per run.
2. A libproc success that lists only non-positive PIDs now gives an empty list instead of `None`.
   Invisible on this host (and any real kernel). `libproc_outcome` no longer filters, so `[0]` or
   `[-1]` is `Pids`, `tally_reads` skips them, and the listing is `Ok(empty)` where A gave `None`
   (`NoGpuSource`). `proc_listpids` always lists launchd (1) and the caller. The empty-list case
   itself is still pinned (`libproc_outcome_an_empty_list_is_failed`, mutation 15a CAUGHT).
3. Found beyond the slimmers' two: the `p_comm` name in the libproc-refused fallback now comes from a
   per-PID `KERN_PROC_PID` lookup, not the `KERN_PROC_ALL` record. Invisible: profile X above gives
   identical names in A, B0 and B; the cost is one more `sysctl` per refused-pidpath row.
