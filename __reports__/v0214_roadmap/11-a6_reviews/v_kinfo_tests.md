# Review of the tests added by `kinfo_enumeration` (task/kinfo_enumeration @ 201a1db, on b716087)

**Verdict: PASS WITH NOTES** (0 blocker, 6 should-fix, 7 note)

Every one of the 34 new tests failed under at least one mutation of the fix it guards (table below). The red commit 578bf07, whose stubs are the leaf's own reverted fix, gives `33 failed; 1 passed`, with 33 `panicked at` and 0 `error[E`. The test module is byte-identical between 578bf07 and the tip. The gaps are elsewhere. Four mutants a reader would expect the suite to catch survive all 129 lib tests: the `proc_pidpath` errno map, the value of `EPERM`, the bridge's row passthrough, and the whole `KERN_PROC_ALL` reader. Two harness fixtures can also pass vacuously. Most of these gaps come from the leaf spec's test list, not from implementer error, as in A2.

Method: a detached worktree at 201a1db (`scratchpad/v_kinfo_tests`, removed at the end). For each mutation, one edit was made to `src/gpu/{metal,kinfo}.rs`, then `cargo test --locked --all-features --lib -- gpu::` was run (60 tests), then `git checkout -- src`. The live and buffer-length mutants were re-run under `--target x86_64-apple-darwin` (Rosetta).

## Findings, most severe first

### 1. should-fix: `pidpath_failure` has no test, so "p_comm used on ESRCH" survives through the errno map
- `src/gpu/metal.rs:819` (`pidpath_failure`). The three `name_after_pidpath_*` tests take a `PidpathFailure` as input, and no test ever calls the classifier that produces one.
- Evidence: mutant `Some(kinfo::ESRCH) => PidpathFailure::Denied` gives `60 passed`, and mutant `Some(kinfo::EPERM) => PidpathFailure::Other` gives `60 passed`. The first is exactly the name flip the leaf exists to prevent: a gone process named from `p_comm`, which resets `watch.rs::process_sample`'s baseline. The leaf's "catches: p_comm used on ESRCH" holds for `name_after_pidpath` only.
- Fix: add `pidpath_failure_names_only_eperm_denied` with literal errnos: `Some(1)` → `Denied`, `Some(3)` → `Gone`, `Some(22)` and `None` → `Other`. A prototype caught both mutants.

### 2. should-fix: the value of `kinfo::EPERM` is pinned by no test (tautological use of the constant)
- `src/gpu/kinfo.rs:65`. Tests at `src/gpu/metal.rs:1425,1636,1674` write `kinfo::EPERM` on the input side, and production compares against the same symbol.
- Evidence: mutant `EPERM: i32 = 2` gives `60 passed`, and all 129 lib tests pass. The value of the new constant is unchecked. ESRCH and ENOMEM are pinned by the older `kinfo::tests` (mutant `ESRCH = 4` is caught by `decide_exists_follows_the_lookup_rule`; `ENOMEM = 13` by `classify_kern_proc_pid_a_record_that_does_not_fit_is_unusable`). The same file already writes the literal `Some(22)` for EINVAL.
- Fix: write `Some(1)` in `footprint_from_errno_eperm_is_denied` and `libproc_outcome_eperm_is_refused`, or in the new `pidpath_failure` test. A prototype caught `EPERM = 2`.

### 3. should-fix: `legacy_entries_keeps_an_empty_list_with_nothing_denied` checks only `is_some()`, so a bridge that drops rows passes
- `src/gpu/metal.rs:1802,1809`, guarding `legacy_entries` at `:1265`.
- Evidence: mutant G6 (`else { Some(Vec::new()) }`) gives `60 passed`. Under profile S the list would then print nothing. Mutant G7 (a bridge that filters out hmn's own 16 KiB row, a plausible "fix" of the self row in the wrong layer) gives `60 passed`.
- Fix: `GpuProcessEntry` has no `PartialEq`, so assert the PIDs: `assert_eq!(legacy_entries(partial).map(|v| v.iter().map(|e| e.pid).collect::<Vec<_>>()), Some(vec![1234]));`. A prototype caught G6.

### 4. should-fix: no test reaches `list_kinfo_all`, and the "arm64-only layout" catch is attributed to tests that cannot make it
- `src/gpu/metal.rs:1007` (`list_kinfo_all`) and `:1704,1712` (`kinfo_all_buffer_len_*`).
- Evidence: mutant W3 (MIB `[CTL_KERN, KERN_PROC, KERN_PROC_PID]`) and mutant W9 (`KERN_PROC_ALL = 1`) each give `60 passed`, natively and under Rosetta. The leaf credits `kinfo_all_buffer_len_*` with catching "a layout assumption that holds on arm64 only". They are pure arithmetic over `KINFO_PROC_SIZE` and behave the same on both architectures. Mutant `KINFO_PROC_SIZE = 656` is not caught by either of them; it is caught by the older layout tests. Only the CLI gates under profile S exercise the `KERN_PROC_ALL` path.
- Fix: add a live test, run natively and under `--target x86_64-apple-darwin` like `kern_proc_pid_record_matches_the_kernel_for_this_process`. It would check that `list_kinfo_all()` is `Records` and holds `me()` with `comm == exe || comm == argv0`, through `comm_of`. A prototype passed unmutated on both targets and caught W3 and W9 on both. Also correct the leaf's "catches" text for `kinfo_all_buffer_len_*`.

### 5. should-fix (harness): `compare_names.py` passes all-null captures and any short prefix
- `__reports__/v0214_part1/harness/compare_names.py:38-60`.
- Evidence, from synthetic inputs:

| Inputs | Output | Exit |
|---|---|---|
| Two captures whose names are all `null` | `compared 2 mismatch 0` | 0 |
| `"com.apple.WebKit.GPU"` vs `""` | accepted | 0 |
| `"com.apple.WebKit.GPU"` vs `"c"` | accepted | 0 |
| `"com.apple.WebKit.GPU"` vs `"com.apple.WebKi"` (a 15-byte ASCII cut) | accepted | 0 |
| An empty overlap | `compared 0` | 1 (correct) |

  So an hmn that names nothing in either run, or that cuts `p_comm` early, passes the D-names gate. The leaf's "catches … a comparison of zero PIDs passing vacuously" holds only for an empty overlap. The cut-at-15 case is caught at unit level by `comm_to_name_returns_a_full_16_byte_name_as_is`; the all-null case is caught only by the separate D row-count gate.
- Fix: count a PID only when its base name is non-null. Treat a null on the profile side as a mismatch. Accept `other == full or other == full.encode()[:16].decode("utf-8", "ignore")`: the exact kernel cut plus `comm_to_name`'s valid-prefix rule, never a looser prefix.

### 6. should-fix (harness): `count_denied.py` prints a passing line on an empty table
- `__reports__/v0214_part1/harness/count_denied.py:78`.
- Evidence: with `list_pids` patched to return `[]`, and again with it returning only the probe's own PID, the probe prints `denied=0 read=0 gone=0` and exits 0. Three checks therefore pass vacuously on an empty list: the `none denied=0` line, Step 4's `grep -c "^denied=0 "` = 1 check, and the P/S0 `read=0` lines. The S line (`denied ≥ 100`, `read ≥ 1`) does catch it. The seam gates in part2_close and ps_watch_unreadable would agree with an hmn that also saw nothing. The probe also never checks that `size.value` is a multiple of 648.
- Fix: exit 1 unless the probe's own PID is among the listed PIDs. That is the probe's analogue of `trust_kinfo_listing`, and it also catches a wrong record size or `p_pid` offset. Exit 1 too when `size.value % 648 != 0`.

### 7. note: `kern_proc_pid_comm`'s "goes through classify_kern_proc_pid" catch is not met
- `src/gpu/metal.rs:1124`, with the test at `:1699`.
- Evidence: mutant K1 (the classify step deleted, `buf.get(..len)` still parsed) gives `60 passed`, natively and under Rosetta. A dead PID gives `rc` 0 and `len` 0, so the parse is empty either way. Only the variant that parses the whole buffer (K2) is caught. The real impact is small: a refused call's zeroed record has an empty comm, and `comm_to_name` turns that into `None`.
- Fix: correct the leaf's claim. Or split out a pure `comm_from_kern_proc_pid(rc, errno, &buf, len, pid)` and test that a refused, zeroed 648-byte buffer gives `None`.

### 8. note: the comment "No syscall is issued for an index below zero" is not tested
- `src/gpu/metal.rs:1455`.
- Evidence: mutant F8+ (the `ledger` call is made for `idx < 0` and `Unavailable` is returned after it) gives `60 passed`. Returning `Gone` (F5, F8) is caught.
- Fix: reword the comment to what the test asserts, or accept it as a residual.

### 9. note: the `b"\x80"` assertion cannot tell the incomplete-tail guard apart
- `src/gpu/metal.rs:1629`.
- Evidence: mutant C5 (any invalid UTF-8 keeps its valid prefix) is caught only by the `b"ab\xffcd"` line. With that line removed (C5'), `60 passed`, because `\x80`'s valid prefix is empty and gives `None` either way.
- Fix: use `b"ab\x80"`, which gives `Some("ab")` under the mutant.

### 10. note: duplicate coverage
- The two `classify_kern_proc_pid(..)` cases in `trust_kinfo_listing_keeps_records_only_when_self_is_one_648_byte_record` (`:1736-1749`) both evaluate to `Unusable`. They are the same check as the `|| PidLookup::Unusable` line: with that line deleted, mutant R9 is still caught by them. They re-test `classify_kern_proc_pid`, which `kinfo::tests` already pin. The leaf mandates them, and they cost little.
- `libproc_outcome(8, None, vec![0, -1])` inside `enumerate_pids_with_falls_back_only_when_libproc_says_eperm` (`:1400`) repeats `libproc_outcome_an_empty_list_is_failed`'s second assertion (`:1663`). Mutants L4, L5 and L6 fail both tests.
- `kern_proc_pid_comm_returns_this_process_name` (`:1684-1695`) copies the 11-line exe/`argv[0]` block of the existing live test verbatim. One helper would serve both.

### 11. note: `kinfo_all_buffer_len_*` restate `KINFO_PROC_SIZE` instead of the literal 648
- `src/gpu/metal.rs:1705,1713`. The leaf states the rule as "a whole number of 648-byte records". Because the tests use the constant, they follow it when it is wrong (mutant K8). The older live test catches K8, so this is cosmetic, but a literal `648` would make the tests stand on their own.

### 12. note: wiring residuals that no unit test can reach
These mutants survive the unit suite:

| Mutant | What it changes | Covered elsewhere |
|---|---|---|
| W1 | The guard is unwired | The leaf's `grep -c 'trust_kinfo_listing(list_kinfo_all()'` gate (acknowledged) |
| W2 | ENOMEM is not retried | Nothing |
| W4 | A partial ENOMEM fill is parsed | Nothing |
| W5 | `list_libproc_pids` drops errno | The profile-P CLI gate |
| W6 | An unresolved index is read as entry 0 | Nothing |
| W7 | `ProcRef.comm` is ignored | Nothing; benign, `KERN_PROC_PID` gives the same bytes |
| W8 | The `p_comm` fallback is removed from `list_processes` | The profile-D gates |

W2, W4 and W6 are the residuals worth stating in the PR body.

### 13. note: the red commit is still a separate commit on the task branch
- 578bf07 carries `#[allow(dead_code)] // stub, removed in Step 2` and the stubs, and fails `cargo test`. SQUASH SCOPE requires it to be folded into 761312a before the PR branch is pushed.

## Mutation table

Test names are those in `gpu::metal::tests`, except `kinfo::` ones from `gpu::kinfo::tests`.

| Mutation | Test that caught it, or "survived" |
|---|---|
| E1 kinfo run first, libproc only if it fails (the stub's order) | enumerate_pids_with_keeps_libproc_pids_and_never_runs_kinfo, enumerate_pids_with_falls_back_only_when_libproc_says_eperm |
| E1b kinfo run eagerly, its result used only on Refused | same two |
| E2 fallback on any libproc failure (`Refused \| Failed`) | enumerate_pids_with_falls_back_only_when_libproc_says_eperm |
| E3 double refusal masked as `Some(vec![])` | enumerate_pids_with_is_none_when_both_refuse |
| E3b only kinfo `Refused` masked as empty | enumerate_pids_with_is_none_when_both_refuse |
| E4 kinfo records lose their `p_comm` | enumerate_pids_with_falls_back_only_when_libproc_says_eperm |
| F1 ESRCH counted as Denied | footprint_from_errno_esrch_is_gone |
| F2 any errno counted as Denied | footprint_from_errno_other_is_gone_never_denied, footprint_from_errno_esrch_is_gone |
| F3 missing errno counted as Denied | footprint_from_errno_none_is_gone |
| F4 EPERM read as Gone | footprint_from_errno_eperm_is_denied |
| F5 unresolved index reported as Gone | footprint_unavailable_when_the_template_index_did_not_resolve |
| F6 negative balance wrapped (`unsigned_abs`) | footprint_from_balance_negative_is_gone_otherwise_bytes |
| F7 zero balance read as Gone | footprint_from_balance_negative_is_gone_otherwise_bytes |
| F8 `idx < 0` goes to the syscall, then Gone | footprint_unavailable_when_the_template_index_did_not_resolve |
| F8+ `idx < 0` goes to the syscall, then Unavailable | **survived** (finding 8) |
| T1 gone PID counted as denied | tally_reads_ignores_gone_pids |
| T2 caller listed as denied | tally_reads_never_lists_the_caller_as_denied |
| T3 zero balance not counted as read | tally_reads_counts_a_zero_balance_as_read, tally_reads_does_not_count_the_callers_own_read_as_another_process |
| T4 zero balance makes an entry | tally_reads_counts_a_zero_balance_as_read |
| T5 own read counted as another process | tally_reads_does_not_count_the_callers_own_read_as_another_process |
| T6 own row dropped from `found` | tally_reads_does_not_count_the_callers_own_read_as_another_process |
| T7 PID 0 not skipped (`pid < 0`) | tally_reads_skips_non_positive_pids |
| T8 non-positive PIDs never skipped | tally_reads_skips_non_positive_pids |
| T9 Unavailable ignored | tally_reads_is_none_when_the_index_is_unavailable |
| T10 Unavailable gives an empty tally | tally_reads_is_none_when_the_index_is_unavailable |
| T11 denied PID counted as read | tally_reads_does_not_count_the_callers_own_read_as_another_process, tally_reads_never_lists_the_caller_as_denied |
| N1 `p_comm` used on Gone | name_after_pidpath_gone_never_uses_comm |
| N2 `p_comm` used on Other | name_after_pidpath_other_is_none |
| N3 comm closure evaluated first | all three name_after_pidpath_* |
| N4 no comm on Denied | name_after_pidpath_uses_comm_only_when_denied |
| N5 `pidpath_failure`: ESRCH → Denied | **survived** (finding 1) |
| N6 `pidpath_failure`: EPERM → Other | **survived** (finding 1) |
| C1 no NUL cut | comm_to_name_cuts_at_nul, comm_to_name_rejects_empty_and_invalid |
| C2 cut at 15 bytes | comm_to_name_returns_a_full_16_byte_name_as_is |
| C3 a cut character gives None | comm_to_name_keeps_the_valid_prefix_of_a_cut_char |
| C4 lossy U+FFFD | comm_to_name_keeps_the_valid_prefix_of_a_cut_char, comm_to_name_rejects_empty_and_invalid |
| C5 any invalid UTF-8 keeps its valid prefix | comm_to_name_rejects_empty_and_invalid (only its `ab\xffcd` line) |
| C5' C5 with the `ab\xffcd` assertion removed | **survived** (finding 9) |
| C6 empty name is `Some("")` | comm_to_name_rejects_empty_and_invalid |
| L1 any libproc failure Refused | libproc_outcome_other_errnos_are_failed_never_refused |
| L2 ENOMEM Refused | libproc_outcome_other_errnos_are_failed_never_refused |
| L3 ESRCH Refused | libproc_outcome_other_errnos_are_failed_never_refused |
| L4 empty success list is `Pids([])` | libproc_outcome_an_empty_list_is_failed, enumerate_pids_with_falls_back_only_when_libproc_says_eperm |
| L5 non-positive PIDs kept | libproc_outcome_keeps_only_positive_pids, libproc_outcome_an_empty_list_is_failed, enumerate_pids_with_falls_back_only_when_libproc_says_eperm |
| L6 PID 0 kept (`>= 0`) | same three |
| L7 errno read even on success | libproc_outcome_keeps_only_positive_pids |
| L8 a failed call keeps its pids | libproc_outcome_other_errnos_are_failed_never_refused |
| L9 EPERM never Refused | libproc_outcome_eperm_is_refused |
| K1 `kern_proc_pid_comm` skips classify, parses `buf[..len]` | **survived**, native and Rosetta (finding 7) |
| K2 `kern_proc_pid_comm` skips classify, parses the whole buffer | kern_proc_pid_comm_is_none_for_a_dead_pid (native and Rosetta) |
| K3 hand-written comm layout at offset 244 | kern_proc_pid_comm_returns_this_process_name (native and Rosetta) |
| K4 stub `Some(Vec::new())` | both kern_proc_pid_comm_* (native and Rosetta) |
| K5 classify against PID 1 | kern_proc_pid_comm_returns_this_process_name (native and Rosetta) |
| K6 `ESRCH = 4` | kinfo::decide_exists_follows_the_lookup_rule |
| K7 `ENOMEM = 13` | kinfo::classify_kern_proc_pid_a_record_that_does_not_fit_is_unusable, kinfo::decide_exists_follows_the_lookup_rule |
| K8 `KINFO_PROC_SIZE = 656` | four kinfo::* tests, kern_proc_pid_record_matches_the_kernel_for_this_process, kern_proc_pid_comm_returns_this_process_name, gpu::tests::process_exists_finds_kernel_task_on_macos; **not** by kinfo_all_buffer_len_* (finding 11) |
| B1 no record rounding | both kinfo_all_buffer_len_* (native and Rosetta) |
| B2 no slack | kinfo_all_buffer_len_is_a_record_multiple_with_slack |
| B3 zero gives zero | kinfo_all_buffer_len_of_zero_is_one_record_slack (native and Rosetta) |
| B4 rounded down | kinfo_all_buffer_len_is_a_record_multiple_with_slack |
| B5 1/16 slack | kinfo_all_buffer_len_is_a_record_multiple_with_slack |
| B6 hard-coded 656-byte records | both kinfo_all_buffer_len_* (native and Rosetta) |
| R1 guard returns the listing unchanged | both trust_kinfo_listing_* |
| R2 guard trusts Unusable | trust_kinfo_listing_keeps_records_only_when_self_is_one_648_byte_record |
| R3 guard trusts `Refused { .. }` | same |
| R4 guard trusts NoRecord | same |
| R5 empty listing kept as `Records([])` | trust_kinfo_listing_never_asks_self_without_records |
| R6 self asked eagerly | trust_kinfo_listing_never_asks_self_without_records |
| R7 Refused turned Failed | trust_kinfo_listing_never_asks_self_without_records |
| R8 self asked for an empty listing | trust_kinfo_listing_never_asks_self_without_records |
| R9 R2 with the `\|\| PidLookup::Unusable` assertion removed | trust_kinfo_listing_keeps_… (its classify cases; finding 10) |
| G1 the `entries.is_empty()` rule | legacy_entries_hides_a_list_where_only_denials_were_found |
| G2 all-denied gives `Some(empty)` | legacy_entries_hides_a_list_where_only_denials_were_found |
| G3 a partial denial hidden | legacy_entries_keeps_an_empty_list_with_nothing_denied |
| G4 stub: always `Some(entries)` | legacy_entries_hides_a_list_where_only_denials_were_found |
| G5 an empty list with nothing denied hidden | legacy_entries_keeps_an_empty_list_with_nothing_denied |
| G6 bridge drops every row it keeps | **survived** (finding 3) |
| G7 bridge drops a 16 KiB (own) row | **survived** (finding 3) |
| W1 guard unwired (`list_kinfo_all` bare) | survived (acknowledged; grep gate) |
| W2 `list_kinfo_all`: ENOMEM not retried | survived (finding 12) |
| W3 `list_kinfo_all`: MIB `KERN_PROC_PID` | **survived**, native and Rosetta (finding 4) |
| W4 `list_kinfo_all`: a partial ENOMEM fill parsed | survived (finding 12) |
| W5 `list_libproc_pids`: errno not passed | survived (profile-P CLI gate) |
| W6 unresolved index read as entry 0 | survived (finding 12) |
| W7 `ProcRef.comm` ignored | survived (benign) |
| W8 `p_comm` fallback removed from `list_processes` | survived (profile-D gates) |
| W9 `KERN_PROC_ALL = 1` | **survived**, native and Rosetta (finding 4) |
| W10 `EPERM = 2` | **survived** (finding 2) |

**Each test, against the mutants it caught:**

| Test | Caught |
|---|---|
| enumerate_pids_with_keeps_libproc_pids_and_never_runs_kinfo | E1 |
| enumerate_pids_with_falls_back_only_when_libproc_says_eperm | E2, E4 |
| enumerate_pids_with_is_none_when_both_refuse | E3 |
| footprint_from_errno_eperm_is_denied | F4 |
| footprint_from_errno_esrch_is_gone | F1 |
| footprint_from_errno_other_is_gone_never_denied | F2 |
| footprint_from_errno_none_is_gone | F3 |
| footprint_unavailable_when_the_template_index_did_not_resolve | F5 |
| footprint_from_balance_negative_is_gone_otherwise_bytes | F6, F7 |
| tally_reads_ignores_gone_pids | T1 |
| tally_reads_never_lists_the_caller_as_denied | T2 |
| tally_reads_counts_a_zero_balance_as_read | T3, T4 |
| tally_reads_does_not_count_the_callers_own_read_as_another_process | T5, T6 |
| tally_reads_skips_non_positive_pids | T7, T8 |
| tally_reads_is_none_when_the_index_is_unavailable | T9, T10 |
| name_after_pidpath_uses_comm_only_when_denied | N4 |
| name_after_pidpath_gone_never_uses_comm | N1 |
| name_after_pidpath_other_is_none | N2 |
| comm_to_name_cuts_at_nul | C1 |
| comm_to_name_returns_a_full_16_byte_name_as_is | C2 |
| comm_to_name_keeps_the_valid_prefix_of_a_cut_char | C3 |
| comm_to_name_rejects_empty_and_invalid | C5, C6 |
| libproc_outcome_eperm_is_refused | L9 |
| libproc_outcome_other_errnos_are_failed_never_refused | L1–L3, L8 |
| libproc_outcome_an_empty_list_is_failed | L4 |
| libproc_outcome_keeps_only_positive_pids | L5–L7 |
| kern_proc_pid_comm_returns_this_process_name | K3, K5 |
| kern_proc_pid_comm_is_none_for_a_dead_pid | K2, K4 |
| kinfo_all_buffer_len_is_a_record_multiple_with_slack | B1, B2, B4, B5 |
| kinfo_all_buffer_len_of_zero_is_one_record_slack | B3 |
| trust_kinfo_listing_keeps_records_only_when_self_is_one_648_byte_record | R2–R4 |
| trust_kinfo_listing_never_asks_self_without_records | R5–R8 |
| legacy_entries_hides_a_list_where_only_denials_were_found | G1, G2, G4 |
| legacy_entries_keeps_an_empty_list_with_nothing_denied | G3, G5 |

## What I checked and found sound

- **Counts.** The leaf's `--list` filters give 3, 4, 1, 1, 6, 3, 4, 4, 2, 2, 2 and 2 tests: 34 in all. The lib holds 129 tests, the 95 of b716087 plus 34. All 129 pass natively and under `--target x86_64-apple-darwin`, and the test binary is an x86_64 Mach-O.
- **The live tests.** `kern_proc_pid_comm_returns_this_process_name`, `kern_proc_pid_comm_is_none_for_a_dead_pid` and both `kinfo_all_buffer_len_*` pass natively and under Rosetta, and their mutants (K2–K5, B1, B3, B6) are caught on both targets.
- **The red commit.** At 578bf07, `panicked at` = 33 and `error[E` = 0, with `33 failed; 1 passed`. The one test that passes is `legacy_entries_keeps_an_empty_list_with_nothing_denied`, as the leaf says, and G3 and G5 show its value.
- **Lint allowances.** The leaf adds no `#[allow]` in test code; the module keeps only b716087's `#[allow(clippy::unwrap_used)]`. Swapped for `#[expect]`, clippy `--all-targets -D warnings` exits 0 with no unfulfilled expectation under five configurations:
  - stable, default features
  - stable, `--all-features`
  - stable, `--all-features --target x86_64-unknown-linux-gnu` (`metal.rs` is not compiled there, so the expectation is moot)
  - `+1.88`, default features
  - `+1.88`, `--all-features`

  The tests do call `.unwrap()`.
- **House idiom.** In `mod tests`:
  - No `cfg!`, `panic!`, `unreachable!`, `.expect(`, `_ =>` arm, `#[ignore]` or `as` cast; `grep -c 'panic!\|_ =>' src/gpu/metal.rs` gives 0.
  - Slices are taken with `.get(..16)`.
  - PIDs go through `i32::try_from(..).unwrap()`, the existing live test's form. The leaf's text asked for "an explicit fallback arm", but `unwrap` fails louder and matches the house.
  - Emptiness is asserted with a `{:?}` message. `assert!(x.is_none())` matches nvidia_smi.rs, snapshot.rs and report.rs.
  - Closures that must not run set a `Cell<bool>` flag (6 flags) rather than panicking.
  - The tests are small and literal.
- **No tautologies beyond findings 2 and 11.** The flag tests are not tautological: R6, R8, N3, E1 and E1b each fail them.
- **`sandbox.sh --job` is not vacuous.**
  - No command after `--job` (with or without `--`) → exit 64.
  - An empty-string command → exit 127.
  - A command's failure is propagated (rc 1).
  - `Z --job` and `--job S` → exit 64.
  - `$JOB` is set and alive inside the S sandbox.
  - A job that does not start → exit 70, by the code's `kill -0` check.
  - The binary is compiled into the gitignored `target/gpujob`.
- **`compare_names.py`** exits 1 on an empty overlap and 2 on an empty file.
- **Clean-up.** The worktree is clean after every mutation (`git status --short` is empty). I removed it at the end.
