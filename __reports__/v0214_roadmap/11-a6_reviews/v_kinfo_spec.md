# v_kinfo_spec: spec verification of task/kinfo_enumeration (tip 201a1db)

Verdict: PASS WITH NOTES (VERIFY_COMPLETE, confidence High). Blocker 0, should-fix 2, note 7.
Every Success Gate and Consistency Check passes on 761312a, e76524b and 201a1db (each checked out on its own), the Rosetta lib run passes (129/129), and 12 of the 13 `catches:` mutations I tried were caught. One was not (finding 1).

## Findings

1. should-fix, src/gpu/metal.rs:1124-1134 (`kern_proc_pid_comm`): the `kern_proc_pid_comm` gate says it catches "a lookup that does not go through `classify_kern_proc_pid`", and no test does.
   Mutation: replaced the `matches!(classify_kern_proc_pid(..), PidLookup::Record)` early return with `let _ = (rc, errno);`. Result: 129 lib tests pass, rc 0, nothing failed.
   Why it survives: the live test reads this process (rc 0, one record) and the dead-PID test gets rc 0, len 0, so the parse alone gives `None`. The refused or ENOMEM case needs a sandbox or a stale buffer and never reaches a unit test.
   Runtime effect today is nil (a zeroed 648-byte buffer parses as pid 0 with an empty `comm`, which `comm_to_name` turns into `None`), so this is a gate-strength defect, not a behaviour one.
   Fix: split the pure part out, e.g. `fn comm_from_lookup(rc, errno, buf, len, pid) -> Option<Vec<u8>>`, and test `(-1, EPERM, zeroed, 648, pid)` and `(0, 0, a record naming another pid, 648, pid)` as `None`.

2. should-fix, __reports__/v0214_part2/kinfo_enumeration.md:5: the record says the binaries are "of the tree at e76524b". The squash of 578bf07 into 761312a re-creates every later commit, so e76524b will not exist on the pushed branch (it is the only SHA of this branch in any committed file).
   Fix: word it "the tree after Steps 1 to 3" (the src tree is identical at 201a1db), or rewrite the SHA when squashing.

3. note, docs/FAQ.md:289: the reflow left one 90-column line (`platforms still render an unresolved row as a bare ...`); the rest of the answer wraps at 72-77. Cosmetic: re-wrap the paragraph.

4. note, spec issue (INFORM): Step 3's wording "a name is `?` only when both `proc_pidpath` and `KERN_PROC_PID` are refused" (FAQ, snapshot.rs) is inexact. `None` also comes from a gone process, a non-UTF-8 or empty comm, and an `Other` pidpath failure. Where enumeration went through `KERN_PROC_ALL`, the comm comes from that listing, not from `KERN_PROC_PID`. The text matches the spec, so it is not a deviation.

5. note, spec conflict (INFORM): the spec limits the harness README edit to the "Extension points for PR C" paragraph, so its files table and usage line still show `sandbox.sh [--print-profile] PROFILE [--] CMD` and omit `gpujob.swift`, `count_denied.py`, `compare_names.py` and exit 70 (documented only in the `sandbox.sh` header). The implementer followed the spec. If the maintainer wants OWNERSHIP applied there, it is a follow-up edit.

6. note, src/gpu/metal.rs:1093 (`enumerate_pids_with`): `LibprocPids::Pids(vec![])` returns `Some(vec![])`. `libproc_outcome` never produces it and the variant doc says "at least one", but the invariant is not enforced, and an empty list would flow to the silent zero. Optional hardening: `Pids(v) if v.is_empty() => None`.

7. note, src/gpu/metal.rs:1007 (`list_kinfo_all`, `list_libproc_pids`): the ENOMEM retry loop and the rc-before-buffer ordering are not unit-tested (they sit behind the syscall). The spec asks for nothing here. Extracting the loop over a closure would let a test pin "4 attempts, then `Failed`, buffer never read on failure".

8. note, src/gpu/metal.rs:1221-1226 (`list_processes`): the lazy `.map(read_graphics_footprint)` issues a ledger read for pid 0 (`kernel_task`, from `KERN_PROC_ALL`) before `tally_reads` skips it. One wasted syscall; no behavioural effect.

9. note, harness: `sandbox.sh S0 --job` runs the same bash wrapper as S, so a sibling exists and S0 under `--job` is not "P-like"; no gate uses it. `gpujob` sleeps 20 s with no trap, so a killed harness leaves it for at most 20 s.

## Gate results (all PASS unless stated)

Re-run exactly as written. Counts of processes are live.

- enumerate_pids_with 3 listed / 3 pass; footprint_from_errno 4; footprint_unavailable 1; footprint_from_balance 1; tally_reads 6; name_after_pidpath 3; comm_to_name 4; libproc_outcome 4; kern_proc_pid_comm 2; kinfo_all_buffer_len 2; trust_kinfo_listing 2; legacy_entries 2. Step 2 check: 34 listed, 34 passed. Lib total 129 (95 + 34). Same on 761312a, e76524b and 201a1db.
- Rosetta (`--target x86_64-apple-darwin --lib`, tip): 129 passed; kern_proc_pid_comm 2/2 and kinfo_all_buffer_len 2/2 on that target.
- S --job: `exit 0`, `1`, `big=0`. D: `exit 0` (23 named of 23 rows; 25-27 rows on re-runs, always all named). compare_names: `compared 21 mismatch 0`, exit 0. S/S0/Q/L: `S 0 0 1`, `S0 2 1 0`, `Q 2 1 0`, `L 2 1 0`. P: `[ps --device 0] exit 2 metal=1 skipped=0 stdout_bytes=0`, `[ps] exit 2 metal=1 skipped=1 stdout_bytes=0`. All the same on 761312a, e76524b and 201a1db (tip harness copied over for the first two).
- count_denied: none `denied=0 read=1050 gone=0`, S `denied=1049 read=1`, P and S0 `read=0`. `Z 64`, `Z-job 64`, `job-first 64`, and the one-line profile P text.
- compare guard on the committed captures: `rc=0`, 2, 43, 43, 2. Fresh same-boot captures of b716087 (built under `target/base`) and of the tip: `none` and `C` identical, 43 rows each, `spill cells mapped: 0`.
- Static: trust token 1; allow(dead_code) `0 0 2`; panic! 0, `_ =>` 0, Cell::new(false) 6; moduledoc KERN_PROC_ALL 1, silently 0, "every call is a libSystem" 0, cross-user 0 (at 761312a the Step 3 tokens are 0, as they should be); at e76524b and 201a1db p_comm `1 1 2`, KERN_PROC_PID `1 1`, KERN_PROC_ALL `1 1`, proc_pidpath in FAQ 1. Step 3 check (p_comm 2, KERN_PROC_ALL 1) passes; Step 4 check passes.
- clippy `--all-targets --all-features -D warnings` on macOS (default and all-features), `--target x86_64-unknown-linux-gnu`, and 1.88: exit 0 on all three commits. Swapping the test module's `#[allow(clippy::unwrap_used)]` for `#[expect]` still compiles clean (the allowance is needed; no other allowance). `cargo fmt --check` 0, `RUSTDOCFLAGS=-D warnings cargo doc --document-private-items` clean, full `cargo test --all-features` 399 passed, `-- --ignored` (cli_ps, live_gpu, macos_smoke) all pass. `cargo package --list --allow-dirty`: 0 lines from `__reports__/`.
- 578bf07 (Step 1 red check): `grep -c 'panicked at'` 33, `error[E` 0, `1 passed; 33 failed`. Its clippy fails on dead code (expected for a red commit that is squashed).
- All 8 Step 4 deliverables and all 34 test names and 23 metal.rs items exist by name.
- BLOCKED: `cargo deny` (not installed here); Cargo.toml, Cargo.lock and deny.toml are unchanged, so no dependency effect.
- "Before" values: b716087 binary through the tip harness gives `exit 2 0 big=1`, 23 rows 1 named, `compared 22 mismatch 22`, `S 2 1 0`, `S0 2 1 0`, `Q 2 1 0`, `L 0 0 0`, P same as after. These match the spec's before-values, so the gates discriminate.

## Mutations (each reverted)

| mutation | caught by |
|---|---|
| fallback to kinfo on `LibprocPids::Failed` | enumerate_pids_with_falls_back_only_when_libproc_says_eperm |
| kinfo run before libproc (spec's prototype) | the 2 of 3 enumerate_pids_with tests the spec names |
| ESRCH and ENOMEM counted `Denied` | footprint_from_errno_esrch_is_gone, ..._other_is_gone_never_denied |
| self read counted as another process | tally_reads_does_not_count_the_callers_own_read_as_another_process |
| zero balance not counted as read | tally_reads_counts_a_zero_balance_as_read and the self-read test |
| caller listed as denied | tally_reads_never_lists_the_caller_as_denied |
| `Unavailable` skipped instead of `None` | tally_reads_is_none_when_the_index_is_unavailable |
| p_comm used on `Gone` and `Other` | name_after_pidpath_gone_never_uses_comm, ..._other_is_none |
| every libproc failure `Refused` | libproc_outcome_other_errnos_are_failed_never_refused |
| empty libproc list `Refused` | libproc_outcome_an_empty_list_is_failed, the fallback test |
| trust `Unusable`, or trust `Refused` | trust_kinfo_listing_keeps_records_only_when_self_is_one_648_byte_record |
| empty `Records` trusted | trust_kinfo_listing_never_asks_self_without_records |
| buffer length not a record multiple | kinfo_all_buffer_len_is_a_record_multiple_with_slack |
| lossy comm, or cut at 15 bytes | comm_to_name_* tests |
| negative balance as bytes; `Unavailable` as `Gone` | footprint_from_balance..., footprint_unavailable... |
| `legacy_entries` tests `entries.is_empty()` | unit test, and the CLI gates (S0 becomes `0 0 1`, P `ps` exit 0 with 96 bytes) |
| bridge replaced by `.map(|l| l.entries)` | CLI: S0 `0 0 1`, L `0 0 0`, P exit 0 |
| no KERN_PROC_ALL fallback | CLI: S --job exit 2, big=1 |
| no `kern_proc_pid_comm` fallback | CLI: D `named=1` of 23, compare_names exit 1 |
| `--job` accepted before PROFILE / before the profile check | harness gate (`job-first 0`, `Z-job 0`) |
| `kern_proc_pid_comm` without `classify_kern_proc_pid` | NOT CAUGHT (finding 1) |

## Stated deviations

- `footprint_from_errno` as if/else: same outcomes as the specified `match`, no wildcard (`_ =>` count 0), all four tests pass, mutation caught. It cannot be a `match` without becoming a `const fn` (clippy). Within spec in behaviour, harmless. Nit: `pidpath_failure` is a `const fn` match, so the pair is inconsistent.
- "empty answer" read as `libproc_outcome(8, None, vec![0, -1])` (becomes `Failed`): within the gate's wording, since the spec's own stub has no empty-`Pids` rule. Harmless; see finding 6.
- `ProcRef.comm` first, then `kern_proc_pid_comm`: the spec says "else"; the code also falls back when the listing's comm was empty or invalid. Same bytes, so it can only give `None` again; it costs one syscall and runs only after `EPERM` from `proc_pidpath`. Harmless.
- Named self-lookup closure: the pinned token appears once, the lookup is lazy and real (`kern_proc_pid_lookup(process_self_pid())`). A stubbed `|| PidLookup::Record` would also pass, as the spec says.
- `sandbox.sh --job` exit 70 (compile failure or job did not start): the spec is silent, 70 collides with neither 64 nor 127, and the header documents it.
- `count_denied.py` ` other=` field: printed only when nonzero (not seen on any run); the gates match the `denied=` prefix. Downstream seam gates must not split on a fixed field count.

## OWNERSHIP and wording

- Updated in the final-state voice (no "now" or "no longer" in any added line): module doc, FFI docs (`ledger`, `sysctl`, `proc_listpids`, `proc_pidpath`), `read_graphics_footprint*`, `read_proc_pidpath_basename`, `list_compute_processes`, the `gpu/mod.rs` Metal-arm comment, `kinfo.rs` module doc and `ENOMEM`, `GpuProcessEntry::name`, FAQ, both `--filter` help texts (confirmed in `hmn ps --help` and `hmn watch --help`), the CONVENTIONS row, and the CHANGELOG `### Added` entry above `### Changed`. The only "still" is in the harness README (`... still exit 64`).
- Left for other leaves by the spec: `ps.rs:586-587` ("when the sandbox withheld `proc_pidpath`", owned by ps_watch_unreadable), README item 9 and `lib.rs`/README capability cells (part2_close).

## Step 4 record and captures

- No raw process lists and no absolute paths in the record, captures or harness (grep for `/Users`, `/private`, `/tmp`, `hacker` over `__reports__/v0214_part2` and the harness: nothing). Captures hold only PIDs and name hashes; 54 files in each of the two directories; the `P` captures are identical between base and tip.
- Values re-measured and consistent with the record: S plain `ps` exit 0 with `hmn: 1 GPU process found (0 MiB committed total).`; S0, P, Q and L plain `ps` exit 2 with the `(skipped)` line; `watch $JOB` names `gpujob` at 256 MiB; `none --job` row `used_bytes` 268451840.
- Residual recorded: under S, plain `ps` lists hmn's own row and exits 0 although nearly everything was denied; the record says it belongs to ps_watch_unreadable.
