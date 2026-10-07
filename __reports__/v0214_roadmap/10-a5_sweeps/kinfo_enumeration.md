# A5 sweep: kinfo_enumeration against b716087

Leaf: `__roadmap__/v0214-sandbox/part1/close/part2/kinfo_enumeration.md` (roadmap worktree, uncommitted).
`dirtree-rdm.sh validate` exits 0 after every edit. Measurement worktree `SCRATCH/m_kinfo_enumeration` (detached b716087), removed afterwards.
`rustup check`: `stable-aarch64-apple-darwin - up to date: 1.99.0`. Nested `sandbox-exec` rc 0.

## Edits

| # | Site (leaf section) | Was | Now | Evidence (command → output) |
|---|---|---|---|---|
| 1 | Pre-cond 1 (part1_close) | paraphrase, no gate quote; no `--job` behaviour | quotes the gate "the harness exists with the interface PR C reuses"; says options are parsed only before PROFILE, so `S --job` runs `--job` (exit 127); 54 pr_b files | `sandbox.sh` on b716087 (option loop before PROFILE only); ke18 run → `exit 127`, stderr `_: --job: command not found`; `find pr_b -type f \| wc -l` → 54 |
| 2 | Pre-cond 2 (process_exists_kinfo) | paraphrased test list; kinfo item list without `ENOMEM`; no `process_self_pid`/`comm_of` | quotes both gate texts; b716087 `kinfo::tests` lists 7 (`classify_kern_proc_pid_*` 4); adds `ENOMEM` (un-cfg'd), `process_self_pid`, `comm_of` | `--list` filters: `kinfo::tests` 7, `classify_kern_proc_pid` 4, `decide_exists` 1, live test 1 native and Rosetta; kinfo.rs:55 `pub(super) const ENOMEM: i32 = 12;` no cfg |
| 3 | Pre-cond 3 (ps_failed_devices) | "PR B behaviour" = PR B tree | quotes (a)/(b); "PR B behaviour" = b716087, measured stderr | `sandbox.sh P -- hmn ps` → exit 2, `(skipped)` line + `hmn: ps: no device could be queried, so nothing could be listed` |
| 4 | Pre-cond 4 (env) | `mod tests` 0 at c1810a5 only | + 95 lib tests native and Rosetta; `mod tests` 1 on b716087 with `#[allow(clippy::unwrap_used)]` only | `cargo test --lib -- --list \| grep -c ': test$'` → 95 / 95 (`--target x86_64-apple-darwin`); `grep -c 'mod tests' src/gpu/metal.rs` → 1; metal.rs:857; `test -x sandbox-exec && test -x swiftc` → 0 |
| 5 | 8 test-count gates (enumerate_pids_with … legacy_entries) | before c1810a5 only | + `b716087 (measured)` 0 listed (each filter, native and Rosetta) | 11 filters `--list` → 0 on both targets |
| 6 | New gate `trust_kinfo_listing` (2 tests) | absent | record-size guard gate (see defect/decision D0) | XNU read + probe below |
| 7 | ke18 gate (S --job) | before: "PR B (from ps_failed_devices) exit 2 …" | b716087 (measured): literal `exit 127`,`0`,`big=1`; with the job started by hand in S's wrapper: exit 2, empty stdout, skip + no-device lines → `exit 2`,`0`,`big=1` | ke18 command → `exit 127 / 0 / big=1`; scratch `jobrun.sh S` → exit 2, stdout empty; unsandboxed job read `used_bytes` 268451840 |
| 8 | D names gate | before 40a701e only | + b716087 (measured) 23 rows, 22 null, 1 named → `exit 1` | gate → `exit 1`; row counts 23/1/22 |
| 9 | compare_names gate | before 40a701e scratch only | + b716087: `exit 2` (script absent), scratch copy `compared 19 mismatch 19`, exit 1 | gate literal → python3 errno 2; `SCRATCH/ke_scratch/compare_names.py` → `compared 19 mismatch 19` |
| 10 | S/S0/Q/L gate | "PR B (from its spec …) the same four" | b716087 (measured) `S 2 1 0`, `S0 2 1 0`, `Q 2 1 0`, `L 0 0 0`, with the stderr line | gate → those four lines |
| 11 | P guard gate | "PR B (from … spec)"; "(guard against PR B)"; after "same as PR B" | b716087 (measured) `[ps --device 0] exit 2 metal=1 skipped=0 stdout_bytes=0`, `[ps] exit 2 metal=1 skipped=1 stdout_bytes=0`; "(guard against b716087)" | gate → those two lines |
| 12 | count_denied gate | before: file absent | + b716087 literal (each line profile + space, `Z 64`, profile P text), and a scratch copy: none 0/1053, S 1053/1, P 1053/0, S0 1053/0 | gate literal; `SCRATCH/ke_scratch/count_denied.py` under each profile |
| 13 | compare.py guard gate | before "PR B against itself, from part1_close's compare.py spec" | b716087 (measured): pr_b vs pr_b gives the 5 values (49/47); pr_b vs fresh b716087 capture `rc=1` (PID reuse after reboot); two same-boot b716087 captures `rc=0`, 39/39 | see D1 |
| 14 | clippy/dead_code gate | before 40a701e; dead_code "on PR B"; catches lists `ENOMEM` as needing cfg | + b716087 exit 0; dead_code 0/0/2 on b716087; catches: main's `ENOMEM` gated or duplicated | `cargo clippy --locked --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings` → rc 0 (native also 0); `grep -c 'allow(dead_code)'` → kinfo 0, mod 0, metal 2 |
| 15 | test-code static gate | `Cell::new(false)` ≥ 5; befores c1810a5/PR B | ≥ 6 (+ guard's flag); befores b716087 0/0/0; prototype's 32 noted, the 2 guard tests unmeasured | `grep -c 'panic!'` 0, `'_ =>'` 0, awk Cell 0 |
| 16 | doc static gate | befores c1810a5/PR B | b716087: KERN_PROC_ALL module doc 0, `silently` 3 (sites named), `every call is a libSystem` 1, `cross-user` 0 | greps → 0, 3 (lines 12, 635, 709), 1, 0 |
| 17 | p_comm/KERN_PROC/proc_pidpath gate | befores c1810a5/PR B; misquoted FAQ sentence ("only when") in catches | b716087 0,0,0 / 0,0 / 0,0 / 1; catches quotes main's sentence | greps on b716087 |
| 18 | Step 1 item 1 | adds `ENOMEM` | drops it, reuses `kinfo::ENOMEM`; `KinfoRead` gets `#[derive(Debug, PartialEq, Eq)]` | kinfo.rs:52-55 |
| 19 | Step 1 item 2 | — | + `trust_kinfo_listing` stub (calls closure, returns listing) | — |
| 20 | Step 1 item 3 | "Add `mod tests` … 32 tests"; "keeps the `#[allow(clippy::unwrap_used, clippy::expect_used)] // test-only`"; 31 of 32 | extend existing module; 34 tests; house rules (no `cfg!`, `#[cfg]` before `#[test]`, `#[ignore]` "requires", none needed here); `#[allow(clippy::unwrap_used)]` only, `#[expect]` check; reuse `comm_of`; the two guard tests specified | metal.rs:856-858 on b716087; CONVENTIONS.md *Test-Module Lint Allowances* |
| 21 | Step 1 item 4 / item 5 / Deliverables / Consistency Check | ≥ 31 red, 32 green, list without guard tests, filter list without `trust_kinfo_listing` | ≥ 33 red (`33 failed; 1 passed` expected), 34, +2 test names, + filter; Deliverables drop `ENOMEM` | — |
| 22 | Step 2 item 2 `list_kinfo_all` | "On ENOMEM (the table grew) retry the probe and fill up to 3 times" | exact XNU behaviour (probe = records + KERN_PROCSLOP; short fill copies whole records, fails ENOMEM, writes back len 0; EPERM leaves len as passed); `rc` first, partial fill never parsed; retried up to 3 times (4 attempts), then `Failed`; `buf.get(..len)` | XNU + probe (below) |
| 23 | Step 2 item 2 (new bullets) | — | rewrite `ENOMEM` doc for both MIBs + module doc names KERN_PROC_ALL; `trust_kinfo_listing` spec and wiring | kinfo.rs:3-10, 52-55 |
| 24 | Step 2 item 5 | "on PR B"; "each `as` carries the repo's `#[allow(...)]` with a reason (as `ps.rs` does)" | b716087 0/0; main's convention: prefer `try_from`, else `// CAST:` comment over a statement `#[allow(clippy::as_conversions, …)]` naming only firing lints, as `kern_proc_pid_raw` (d2d26bb); 3-element MIB note; SAFETY: `sysctl` is BSD (`<sys/sysctl.h>`), not POSIX | metal.rs:783-789, 51-57; CONVENTIONS.md *CAST Annotation*; `git show d2d26bb` |
| 25 | Step 2 item 6 | "part1_close already replaced cross-user … for PR B" | + PR B's review; 0 on b716087; module doc's "Process ownership plays no part" | grep → 0; metal.rs:11-13 |
| 26 | Step 2 Deliverables / Consistency Check | `ENOMEM` kept cfg-gated; `= 32`; no guard | `ENOMEM` doc rewritten, not gated; `trust_kinfo_listing`; `= 34` + filter | — |
| 27 | Step 3 item 2 (FAQ) | quoted "a macOS name is `?` only when a sandbox withheld `proc_pidpath`"; "1 on PR B"; one other `On macOS` anchor | main's exact sentence (hard-wrapped after `` `?` ``); 1 on b716087; two other `On macOS` lines named | FAQ.md:286-288, 298, 307 |
| 28 | Step 3 item 5 (CHANGELOG) | "under `### Added`" | `[Unreleased]` has only `### Changed`/`### Fixed`: add `### Added` above `### Changed` | awk over CHANGELOG → `### Changed`, `### Fixed` |
| 29 | Step 4 item 1 gpujob | c1810a5 only | + b716087 scratch build: `gpujob`, 268451840 | scratch `gpujob.swift` + `jobrun.sh none` |
| 30 | Step 4 item 1 `--job` | "exits 64 on an unknown option, so this is an addition, not a check; it goes after PROFILE" | main's actual parser, the after-PROFILE form kept, exact change (consume one `--job` after the profile check, before the `--` strip; `Z --job` still 64; `--job` before PROFILE still 64; one `bash -c` in one `sandbox-exec`, `none` unsandboxed, `C` `-f codex.sb`) | `sandbox.sh` source; ke18 → 127 |
| 31 | Step 4 item 3 | "After PR B, S, S0, P and Q plain ps exit 2 …" | b716087 (measured): S/S0/P/Q plain exit 2 with skip line, L plain exit 0 `0 GPU processes found.`, the S/S0/Q/L table, D 23 rows/22 nameless | plain `ps` under each profile |

31 edit sites.

## D0, the decision the coordinator asked for (record-size guard): `trust_kinfo_listing`

`fn trust_kinfo_listing(listing: KinfoRead, self_lookup: impl FnOnce() -> PidLookup) -> KinfoRead` in metal.rs, pure. A non-empty `Records(v)` stays `Records(v)` only when `self_lookup()` (wired as `kern_proc_pid_lookup(process_self_pid())`) is `PidLookup::Record`. `NoRecord`, `Refused { .. }` and `Unusable` give `Failed`. `Refused` and `Failed` pass through without calling the closure.

One addition beyond your shape, so you can keep it or drop it: `Records(vec![])` is `Failed`, and in that case the closure is not called. `libproc_outcome` already treats an empty list as `Failed`. An empty KERN_PROC_ALL listing would otherwise go through the tally as `Some(empty)`, and the bridge would print `0 GPU processes`, the silent zero.

Tests (+2): `trust_kinfo_listing_keeps_records_only_when_self_is_one_648_byte_record` and `trust_kinfo_listing_never_asks_self_without_records`. The second uses a Cell flag.

Counts are now consistent across the leaf:
- Step 1 item 3: 34 tests.
- Item 4: ≥ 33 `panicked at`, `33 failed; 1 passed`.
- Step 2 check: `= 34`.
- Deliverables: +2 names.
- New header gate: filter `trust_kinfo_listing` → 2.
- Cell gate: ≥ 6.

**part2_close's "32 added by kinfo_enumeration" must become 34.** The lib after this leaf is 95 + 34 = 129 on both targets.

Why this guard, and not a cheaper one: `classify_kern_proc_pid` grants `Record` only to exactly one 648-byte record carrying the right PID. A larger kernel record gets ENOMEM, then `Unusable`. A smaller one fails as a partial record, then `Unusable`. So the guard covers both size directions and the `p_pid` offset, and it is deterministic.
- A `probed % 648` check, or a check for self in the listing, only catches the mismatch by chance: about 1 in 81 for 656-byte records.
- Measured on this host: the self lookup is `rc 0, len 648, pid ok` under none, P, S, S0, D, L and Q. So the guard changes none of the leaf's measured after values.

## XNU facts (xnu-12377.1.9, `bsd/kern/kern_sysctl.c`, `kern_newsysctl.c`, fetched with `gh api`)

- **`sysdoproc_callback`.** Copies one record per process while `buflen >= sizeof_kproc` (user64 = 648 on arm64 and on x86_64 under Rosetta). It always adds `sizeof_kproc` to `needed`.
- **`sysctl_prochandle` with a buffer.** Sets `req->oldlen = dp - where`, the bytes copied, always whole kernel-size records. If `needed > oldlen` it returns ENOMEM, before `req->oldidx += req->oldlen`.
- **`sysctl_prochandle` with a NULL buffer (the probe).** `oldlen = needed + KERN_PROCSLOP` (`5 * sizeof(struct kinfo_proc)`), and it returns 0.
- **`sys_sysctl` / `userland_sysctl`.** On error 0 or ENOMEM they write back `oldidx` (0 when the handler returned ENOMEM). On any other error they return without writing `*oldlenp`.

So on a short buffer KERN_PROC_ALL copies the whole records that fit, then fails with ENOMEM and `len` 0. Measured with a ctypes probe (`SCRATCH/ke_scratch/kproc_probe.py`):
- **3-record buffer:** rc -1, errno 12, len 0, and the buffer holds 3 records.
- **100-byte buffer:** ENOMEM, len 0, nothing copied.
- **Probe:** 687528 = 1061 × 648, against a fill of 1056, so exactly the 5 spare records.
- **Profile Q:** EPERM, and `len` stays as passed.

Main's `ENOMEM` doc ("copies nothing and writes back a len of 0") is true only for KERN_PROC_PID, so the leaf now tells Step 2 to rewrite it.

A4's KERN_PROC_PID point holds as stated. With a probe-sized buffer, a larger-record kernel never answers ENOMEM to KERN_PROC_ALL; hence D0.

## Spec defects not fixed

- **D1 — the compare.py guard gate cannot pass after a reboot.** It compares the committed `__reports__/v0214_part1/pr_b` with the leaf's fresh capture. `compare.py` matches table rows by PID and compares NAME hashes, so any PID that a reboot reassigned shows up as a DIFF. Evidence:
  - The host booted Oct 6 22:03; `pr_b` was committed Oct 5.
  - `compare.py pr_b <fresh b716087 capture>` → rc=1. The only differences are `ps.stdout` and `ps_device_0.stdout` at PID 669: `cc978d28` (ControlCenter) against `ba40ff24` (Finder), under both `none` and `C`.
  - Two same-boot b716087 captures → rc=0, rows 39/39.
  - Proposed fix: in Step 4 item 2, capture b716087 in the same boot (release build from a detached b716087 worktree) into `__reports__/v0214_part2/captures/b716087`, or into a scratch dir outside the repo. Point the gate's base at that capture instead of `pr_b`. The before then becomes "b716087 against itself" (the five values), and the after is b716087 against kinfo_enumeration.
  - I left the gate command and Step 4 item 2's base unchanged, and recorded the measured before.
- **D2 (minor, flagged only) — the ke18 before moves for two reasons.** On b716087 it is `exit 127` only because `--job` does not exist yet. Its code-change before, `exit 2`, comes from a scratch emulation of `--job`. Both are recorded. The gate is moving either way, so no relabel is needed.

## Commands used to measure b716087 (in `SCRATCH/m_kinfo_enumeration`, release = `cargo build --release --locked`, hmn 0.2.13)

- `cargo test --locked --all-features --lib [--target x86_64-apple-darwin] [FILTER] -- --list 2>/dev/null | grep -c ': test$'`. Output: 95 and 95 for the whole lib; 0 for each of the leaf's 12 filters on both targets; `kinfo::tests` 7, `classify_kern_proc_pid` 4, `decide_exists` 1, `parse_kinfo_records` 2, the live test 1, `metal::tests` 1.
- `cargo test --locked --all-features --lib [--target x86_64-apple-darwin]` → `95 passed` on both.
- `grep -c` over `allow(dead_code)`, `panic!`, `_ =>`, `silently`, `every call is a libSystem`, `-ci cross-user`, `p_comm`, `KERN_PROC_PID`, `KERN_PROC_ALL` and `proc_pidpath` on the leaf's files. Output: 0/0/2, 0, 0, 3, 1, 0, 0/0/0, 0/0, 0/0, 1. The awk counts for Cell and the module-doc KERN_PROC_ALL are 0 and 0.
- Every CLI gate exactly as written in the leaf, under `bash -c` from the worktree root. compare_names and count_denied were also run with scratch copies in `SCRATCH/ke_scratch/`.
- The S job was emulated with `SCRATCH/ke_scratch/jobrun.sh` and `gpujob` (`swiftc -O`).
- `HMN_CAPTURE_RAW=SCRATCH/ke_raw python3 harness/capture.py target/release/hmn SCRATCH/ke_capture_b716087` (54 files), then a second capture `_2`. `compare.py` was run as pr_b vs fresh, fresh vs fresh_2, pr_b vs pr_b, and c1810a5 vs pr_b (rc 1).
- `cargo clippy --locked --all-targets --all-features [--target x86_64-unknown-linux-gnu] -- -D warnings` → rc 0 and rc 0.
- `gh api repos/apple-oss-distributions/xnu/contents/bsd/kern/{kern_sysctl.c,kern_newsysctl.c}` (HEAD f6217f8, xnu-12377.1.9).
- `sandbox.sh <profile> -- python3 SCRATCH/ke_scratch/kproc_probe.py` for none, P, S, S0, D, L and Q.
