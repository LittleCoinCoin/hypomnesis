# A5 adversarial verification (commit 0d1cbcd, branch v0214-roadmap, base b716087)

## Verdict: PASS WITH NOTES

Zero blockers, nine notes. I found no tautological gate, no gate that cannot pass on b716087 plus its leaf's own change, and no misquote of a done PR B leaf. Every `b716087 (measured)` before I re-ran matched. The notes are gates that cannot see a mutation their `catches:` claims, one gate that can fail a correct implementation, two cross-leaf stale pre-states, and unspecified detail in the `--job` design.

Method: a detached b716087 worktree of my own (removed), `cargo build --release --locked`, `cargo test --all-features --no-run`, the harness under `bash -c`, and the roadmap files read in full. The roadmap worktree was not touched.

## Findings (most severe first)

### N1. ps_watch_unreadable, static gate "the `run_ps` call site, pinned by grep": the `catches:` overclaims (note)

The gate pins the literal `ps_exit_code(exit_status, rows.is_empty(), failed, notes.unreadable)`. Its `catches:` says it rejects "the unfiltered `listing.denied_pids.len()` instead of the `--pid`-narrowed count, which no run on a one-GPU machine distinguishes".

- The grep cannot see how `notes.unreadable` is accumulated. `notes.unreadable += listing.denied_pids.len()` (unfiltered) keeps the pinned line intact and passes.
- "No run distinguishes it" is false. The profile-S gate with `pid=4294967295` expects `exit 1 unreadable=0`. An unfiltered count gives about 975 and exit 2, so a run does fail on it.
- The same run also catches the `0` placeholder, since WindowServer would exit 1 instead of 2.
- So the mutation is caught by another gate, not by this one.

Fix: reword `catches:` to the call-site mutations the grep sees (`0`, a swapped argument). Add that the narrowed accumulation is pinned by `relevant_denied_*` and by the S/`--pid` gate.

### N2. ps_watch_unreadable, static guard "one remedy constant, one remedy fn, no new `remedy_text` test": the diff grep counts added lines (note; can fail a correct implementation)

The command is `git diff -U0 b716087 -- src/bin/hmn/ps.rs | grep -c "^[-+] *\"[0-9]* GPU processes* found.*re-run elevated for names"`, expected 0.

- The pattern also matches added `+` lines. A new test that asserts a full Windows/Linux summary on one line trips it.
- `format_ps_summary_unreadable_zero_changes_nothing` is specified as "the PR B text, byte for byte". It calls `format_ps_summary_with(.., false)` or `(.., true)`; the leaf allows either.
- Written as `"1 GPU process found (0 MiB committed total; 2 protected — re-run elevated for names)."`, it matches the regex, the gate reads 1, and the implementer is stopped by a gate that is guarding the wrong thing.
- The intent is "no pinned literal is edited or removed". The six pinned literals are guarded separately by the 6-test list.

Fix: grep only `^-` lines (`^-  *\"…`). Or make the zero test use `true`, or call `remedy_clause`, and say so.

### N3. kinfo_enumeration, gate "`sandbox.sh Z -- /usr/bin/true` ... `Z 64`" and Step 4's Consistency Check: the claimed mutation is not exercised (note)

Its `catches:` says "a `--job` parser that swallows an unknown profile". Both the gate and Step 4's check run `sandbox.sh Z -- /usr/bin/true`, with no `--job` on the command line.

- No `--job` code path runs, so that mutation passes both.
- Step 4 specifies two further behaviours that are never run: `sandbox.sh Z --job -- CMD` still exits 64, and a `--job` before `PROFILE` still exits 64.
- I confirmed the current parser: `profile_text` fails before anything else, so `Z --` exits 64 whatever follows.

Fix: add `{ bash $K/sandbox.sh Z --job -- /usr/bin/true 2>/dev/null; echo "Z-job $?"; }` expecting 64. Add `{ bash $K/sandbox.sh --job S -- /usr/bin/true 2>/dev/null; echo "job-first $?"; }` expecting 64, to the same gate and to Step 4's check.

### N4. gpu_process_listing Step 3 item 6 and ps_watch_unreadable Step 2 item 6, CHANGELOG `### Added`: a stale pre-state (note)

- gpu says: "`## [Unreleased]` (on b716087 it holds `### Changed` and `### Fixed` only, measured): under a new `### Added`, placed before `### Changed`".
- kinfo_enumeration Step 3 item 5 runs first in the same PR C and already creates `### Added`. Following gpu literally gives two `### Added` headings in `[Unreleased]`.
- ps_watch_unreadable says only "`### Added` entry", which is ambiguous.
- No gate counts headings. part2_close counts only `^- \*\*` bullets.

Fix: gpu and ps_watch say "reuse the `### Added` heading kinfo_enumeration created; create it only if absent". Add a static check, `sed -n '/^## \[Unreleased\]/,/^## \[0\.2\.13\]/p' CHANGELOG.md | grep -c '^### Added'` equal to 1, to part2_close Step 1 item 3.

### N5. kinfo_enumeration, `trust_kinfo_listing`: the wiring is not pinned by any leaf gate (note)

- The two unit tests cover the function. The CLI gates measure no behaviour change from the guard: "the guard changes no measured outcome".
- An implementation that never calls `trust_kinfo_listing` from `list_processes`, or passes a constant `|| PidLookup::Record`, passes every leaf gate.
- Only an unused function would be caught, and only by the shared clippy dead-code gate. `grep -c 'allow(dead_code)'` stays 2 because the implementer may not add an allow.
- A constant closure also compiles clean.

Fix: add a static pin, `grep -c 'trust_kinfo_listing(list_kinfo_all(), || kern_proc_pid_lookup(process_self_pid()))' src/gpu/metal.rs`, before 0 and after 1, with `catches: guard unwired or stubbed`. The leaf already fixes that exact text in Step 2 item 2. The line is 99 columns plus indent, so rustfmt may wrap it. Pin the short token `trust_kinfo_listing(list_kinfo_all()` instead.

### N6. kinfo_enumeration Step 4, `--job` and `compare_names.py`: details left open (note)

- **`--job`:** "compiles `gpujob.swift` on first use with `swiftc -O`" does not say where the binary goes or whether compilation happens outside the sandbox.
  - If it lands in `__reports__/v0214_part1/harness/`, an untracked binary sits beside committed files and can be picked up by a `git add`.
  - Compiling inside profile S would run `swiftc` under the sandbox.
  - Fix: say "compile outside the sandbox, before `sandbox-exec`, into `target/gpujob`".
- **The note "no profile denies a profile's own self":** profile L (`(deny process-info-ledger)` with no `target self` allow) does deny the caller's own ledger read. I measured `ledger(4, self)` under L: rc -1, errno 1.
  - The claim is about libdispatch crashing Foundation and Metal programs, and no gate runs `--job` under L, so it is harmless. But it contradicts SANDBOX HARNESS, which describes L as "the caller included".
  - Fix: weaken the sentence to "no profile used with `--job`".
- **`compare_names.py`:** the rule "equals the first's or is exactly 16 bytes and a prefix of it" is too strict for a UTF-8-cut name.
  - Step 2 specifies `comm_to_name` to return the valid prefix of a name cut inside a multibyte character, so the profile-D name can be 15 bytes, not 16. The script as written reports a mismatch.
  - The script's behaviour when zero PIDs are common is unspecified: a vacuous exit 0 on an empty overlap.
  - Fix: "a prefix of at most 16 bytes", and "exit 1 when fewer than one PID was compared".
  - I measured all 1113 processes with a path on this machine, comparing `p_comm` with the `proc_pidpath` basename: 0 mismatches. The gate is currently safe; the spec is the issue.
- **`harness/README.md` "Extension points for PR C":** it says PR C runs `compare.py` against the committed `pr_b/` captures and lists only `gpujob.swift` and `count_denied.py`. A5 changed the compare base and Step 4 adds `compare_names.py`. Step 4 forbids editing any other harness file, so the README is left contradicting the leaf.
  - Fix: allow a one-paragraph README update, or call the README superseded.

### N7. Spurious-failure risks from live process lists (note)

- kinfo_enumeration profile-D gate: `test "$names" = "$rows"` over all rows. A GPU-holding process that exits between the ledger read and the name lookup gives `"name":null`, because `Gone` resolves to `None`. This is rare, but nothing in the gate retries.
- The seam gates (ps_watch_unreadable, part2_close) allow ±5 between two sequential sandbox runs. This is fine on a quiet machine. A build or an indexer running in the background could exceed it. 0 was measured.
- gpu_process_listing static gate `git diff b716087 -- src/gpu/...`: README Context says PR C is "rebased onto `main` if `main` moves". A moved `main` that touches those backend files makes the guard non-zero spuriously.
  - Fix: diff against `git merge-base HEAD <upstream main>`.

### N8. part2_close Step 3, `= 7` pattern over the whole R01 (note)

`grep -c -E '\| (listed|partial|blind|unchanged)[^|]* \|$' docs/roadmap-v0.2.14.md` is expected to be exactly 7. It is not scoped to the When-it-bites table.

- The new Verification rows are written by Step 3 itself, with an Evidence cell that is "the quoted line that matters".
- An Evidence cell that happens to start `unchanged` or `blind` is counted and the gate reads 8 or more.
- Fix: scope it with `sed -n '/^\*\*When it bites/,/^---$/p'`, or require the Verification table's last cell not to start with those words.

### N9. gpu_process_listing gate "(guard) gpu_processes_keeps_its_signature": nothing to fix, recorded for completeness

It passes by construction. The leaf says so and gives the mutation (E0308). It sits beside moving gates. This meets GATE CONTRACT (9).

## What I checked and found sound

### Filter counts and arithmetic (measured on b716087 with `--list`)

- Lib 95 and bin 248 tests. `format_ps_summary` lists 28 and the six pinned Windows/Linux names list 6.
- `ps_exit_code` lists 1 (the table test), so 1 + 6 new = 7. `failure_detail`, `remedy_clause`, `relevant_denied`, `denied_pid_notices`, `unreadable_clause` and `sample_failed_line` list 0 on main. `missing_pid_notices` lists 1, so 1 + 1 new = 2.
- No new test name falls under another leaf's filter. `ps_exit_code_empty_and_a_relevant_pid_denied_is_two` contains `relevant_pid_denied`, not `relevant_denied`.
- Union checks: ps Step 2 is 7 + 4 + 4 + 3 + 3 = 21, ps Step 3 is 2 + 2 + 2 + 2 = 8, gpu is 5 + 3 + 1 = 9, kinfo is 3 + 6 + 6 + 3 + 4 + 4 + 2 + 2 + 2 + 2 = 34.
- Lib total 95 + 34 + 9 − 2 = 136, which matches part2_close.
- `metal::tests` lists 1 test, `kinfo::tests` lists 7, `remedy_text` lists 2, the seam test lists 1. `cli_ps` lists 3 with 1 ignored. `macos_smoke` lists 3 ignored.
- The kinfo filters (`enumerate_pids_with`, `footprint_*`, `tally_reads`, `name_after_pidpath`, `comm_to_name`, `libproc_outcome`, `kern_proc_pid_comm`, `kinfo_all_buffer_len`, `trust_kinfo_listing`, `legacy_entries`) and the gpu filters (`process_list_denied`, `decide_listing`) list 0 on main. `kern_proc_pid_comm` does not match the existing `kern_proc_pid_record_matches_…`.
- Doctests: `--doc GpuProcessListing` gives `0 passed ... 5 filtered out` on main, as quoted.
- The red-count arithmetic holds against the stubs. kinfo: 33 of 34 fail; the one pass is `legacy_entries_keeps_an_empty_list…`. gpu: 9 red. ps Step 1: 14 red, with the guards named. ps Step 3: 7 red.

### CLI befores re-run under the harness on b716087

- S/S0/Q/L `ps --device 0`: `S 2 1 0`, `S0 2 1 0`, `Q 2 1 0`, `L 0 0 0`.
- P guard: `[ps --device 0] exit 2 metal=1 skipped=0 stdout_bytes=0` and `[ps] exit 2 metal=1 skipped=1 stdout_bytes=0`.
- D profile: `exit 1` (20 rows at this run; the count follows the live process list).
- The `S --job` gate: `exit 127`, `0`, `big=1`, stderr `_: --job: command not found`.
- gpu CLI gates: P/S0/L `ps --device 0` gives `2 0 0 1`, `2 0 0 1`, `0 0 0 0`. S0 and L plain `ps` give `S0 2 0 1` and `L 0 0 0`. S plain gives `exit 2 skipped=1 found=0 selfrow=0`.
- ps_watch gates: `ps --pid` under S gives `pid=391 exit 2 unreadable=0` and `pid=4294967295 exit 2 unreadable=0`. P/S0 plain and `--device` give 0 for the denial count. L gives `[ps] 0 0` and `[ps --device 0] 0 0`. The five watch lines give `P exit 2 0`, `S auto exit 2 …`, `S min exit 2 0`, `S follow exit 2 unreadable=0` and `S min follow exit 2 …`.
- part2_close gate for the six P/S0/L runs: six exit codes `2 2 2 2 0 0`, `denial text in 0 of 6 files`.
- `macos_sandbox` gate: `0`, `exit 101`, `0`, `inherited exit 101 0`, `sandboxed exit 101 0`.
- `sandbox.sh P -- cargo test --test cli_ps`: `exit 0`, `2`, `0`. Log lines `cli_ps: with --exit-status branch=skipped-device` and the same without.
- `cargo check --no-default-features` and the library-only set: `rc=0 warnings=0`. `cargo test --no-default-features --lib --no-run` builds.

### Same-boot compare base

- `compare.py pr_b pr_b` prints `rc=0`, 2, 49 and 47, 2. Two same-boot b716087 captures give `rc=0`, 2, 39 and 39, 2.
- `pr_b` against a fresh capture gives `rc=1` with PID 669 `cc978d28` against `ba40ff24`. The A5 design is therefore necessary and sufficient.
- `c1810a5` against `pr_b` without `--spill-map` exits 1, so `spill cells mapped: 0` is discriminating.
- The `git worktree add --detach "$PWD/target/base"` nesting is sound: `Cargo.toml` has no `[workspace]`, and `/target` is gitignored.

### `ENOMEM`, `trust_kinfo_listing` and XNU behaviour

- `kinfo.rs:52-55` holds an un-cfg'd `ENOMEM = 12`, read by `classify_kern_proc_pid`'s `Unusable` arm. Reuse is right and its doc is true only for `KERN_PROC_PID`.
- I reproduced the XNU facts with ctypes: probe 710856 bytes (1097 records) against a fill of 707616 (1092), the 5-record `KERN_PROCSLOP`. A 3-record buffer gives `rc -1`, `errno 12`, `len 0`, holding 3 records.
- `trust_kinfo_listing`'s test cases map exactly onto `classify_kern_proc_pid`: `(−1, ENOMEM, …)` is `Unusable`, and `(0, 0, [0;648], 640, me)` is `Unusable` because 640 bytes is a partial record. The 1-in-81 figure for 656-byte records is right (656 × 81 = 648 × 82).
- The guard is deterministic and sound for both directions of size drift.
- Template resolution (`ledger` command 2) works under L, so L produces `Denied`, not `Unavailable`. The after-state `ProcessListDenied` under L holds.

### `count_denied.py` shapes

I reproduced them with an independent ctypes probe: `none denied=0 read=1109`, `S denied=1109 read=1`, `P` and `S0` `denied=1109 read=0`. This matches the gate's expected shapes (denied ≥ 100, read ≥ 1 under S, read=0 under P and S0).

### Other facts checked

- **Gate 21's premise:** a ctypes comparison of `p_comm` against the `proc_pidpath` basename found 0 mismatches over 1113 processes.
- **`gpu_processes`' two `#[allow]`s:** swapping them for `#[expect]` under no-default, default and all features gives "this lint expectation is unfulfilled" ×2 each time. This was reverted.
- **`sort_by_pid` cfg and the `sandbox.sh` parser:**
  - b716087 gates `sort_by_pid` on backend features only, while `decide_listing` is gated `any(macos+metal, test)`. E0425 under `--no-default-features` test is therefore real, and the guard gate sees it.
  - `sandbox.sh`'s parse loop breaks at the first non-dash argument, so `S --job --` runs `--job` as the command.
- **Source anchors:**
  - `ps.rs` 549, `watch.rs` 1186 and 2188 are right.
  - The three `SummaryNotes` literals are at 1269, 1404 and 1417.
  - `tests/smoke.rs` already has the `#[cfg(target_os = "macos")]` and `#[cfg(not(...))]` split in its `Err` arm.
  - `mod tests` exists in `metal.rs` and `error.rs`, one each.
  - `format_vram` floors, so 256 MiB + 16 KiB reads `256 MiB`.
  - `hmn ps --json` and `--json --pid 1` match the quoted key list and shape.
  - `hmn watch` rejects `--filter`, `--min` and `--follow-new` with explicit PIDs, so no gate combines them.
- **Static counts all as quoted on b716087:**
  - `silently` 3, `every call is a libSystem` 1, `cross-user` 0, `allow(dead_code)` 0/0/2, `panic!` 0, `_ =>` 0, `p_comm` and `KERN_PROC_ALL` and `KERN_PROC_PID` 0.
  - `proc_pidpath` in FAQ 1, `four dispatchers` 1, `explained_by_denial` 0, `skipped-device-nogpu` 0.
  - `__reports__`: 145 files and 31 lines, 4 CHANGELOG, 1 ROADMAP, 2 dogfooding, 23 R01, 1 `kinfo.rs`, with the R01 line list exact (68 … 410).
  - README: `the sandbox decides` 1, `denies only` 1, `refuses .proc_listpids` 1. The CHANGELOG `^- \*\*` count between the `[Unreleased]` and `[0.2.13]` headings is 6. R01 ✅ rows 7, `pending, PR C` 4, `only together with the denial line` 1.
- **Shell syntax:** all 36 `bash -c '…'` gate bodies pass `bash -n`. Counted-output captures are not taken after a pipeline, and `$?` is read right after the command where it is used. Padded `wc -l` values are compared against identically padded values.
- **Phrase gates on hard-wrapped files** (GATE CONTRACT 10) grep short tokens or say "one line".
- **Cross-leaf quotes** match PR B: process_exists_kinfo's two gate texts, ps_failed_devices' four commands, the `ps_exit_code_table_pins_the_exit_status_rule` text, `accept`/`skipped_device_line`, remedy_macos's seam and the `(filter first proven to match 2 …)` text, maintainer_review_b's `RemedyPurpose` gate, `68007ba`, `42a0a32` and `d2d26bb`. They also match part1_close's harness gate and its 36 `.exit` files. The PR C leaves quote each other's gates by command or text.
- **Test-idiom compliance:** `#[cfg]` before `#[test]`; "requires" ignore reasons; `Cell<bool>` flags, no `panic!`, no `_` arms, `.get(..)`, `try_from`.
- **`tests/macos_sandbox.rs` design:** implementable and able to fail. The parent and child roles, the probe profile, the fail-not-skip messages, five `^LISTING ` lines, and the inherited-variable and in-sandbox failures are each gated. The doctest gate cannot be fooled by a missing re-export.
- **Release builds:** every CLI gate builds default-features release, so no `[nvidia-smi debug]` line appears.
