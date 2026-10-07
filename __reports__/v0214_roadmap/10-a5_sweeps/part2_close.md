# A5 sweep: part2_close against b716087

Leaf: `__roadmap__/v0214-sandbox/part1/close/part2/api/cli/close/part2_close.md` (roadmap worktree).
Measured in a detached worktree at b716087 (`SCRATCH/m_part2_close`, removed afterwards), with
`cargo build --release --locked` (default features), stable 1.99.0 (`rustup check`: up to date),
nested `sandbox-exec` rc 0. `dirtree-rdm.sh validate` exits 0 after every edit. Not committed.

## Edits (24)

| # | Site (leaf section) | Was | Now | Evidence (command → output) |
|---|---|---|---|---|
| 1 | Pre-cond 2 (harness) | profiles listed, no parser or capture-count note | adds: options only before `PROFILE` (`--print-profile`), unknown option/profile exits 64; `pr_b/` holds 18 `.exit`, 36 with `c1810a5/` | `sandbox.sh` source on b716087; `find pr_b c1810a5 -name '*.exit' \| wc -l` → 36 |
| 2 | Pre-cond 9 (environment) | c1810a5 values only | adds b716087: macos_smoke ignored 3, `__reports__` 145 files, lib 95 native and Rosetta | see the commands below |
| 3 | Gate 1 (README tokens) | "0 on PR B", "1 on PR B" ×3 | `b716087 (measured)`: headline 0, `the sandbox decides` 1, `denies only` 1, `refuses .proc_listpids` 1 | `tr … \| grep -o … \| wc -l` → 0; `grep -cF` → 1, 1; `grep -c` → 1 |
| 4 | Gate 2 (pointer sites) | "the pointer present on PR B" | 0,0,0,0 headline; `Limitations, item 9` FAQ 2, `main.rs` 3 | loop → 0 ×4; `grep -c 'Limitations, item 9' docs/FAQ.md src/bin/hmn/main.rs` → 2, 3 |
| 5 | Gate 3 (Fallback cell) | PR B cell quoted as `…then sysctl`; "1 and 1 on PR B" | b716087 cell has `` `sysctl` `` in backticks; `enumeration, names…` 0,0; `process lookups…` 1,1; `not retried` 0,0 | `grep -c` on README.md, src/lib.rs (README:523, lib.rs:21) |
| 6 | Gate 4 (cross-user etc.) | "on PR B" ×5 | b716087: cross-user README 1, FAQ 1, src files 0 each; `always succeed` 0,0; `sudo hmn ps` 0 ×4; `sudo` 1 (README:505, `sudo kill`) | `grep -c -i 'cross-user' …`; `grep -c …` |
| 7 | Gate 5 (`outside the sandbox`) | "on PR B ≥ 1 each" | 1, 1, 1 at b716087 | `grep -c 'outside the sandbox' README.md docs/FAQ.md docs/tutorials/watching-a-running-job.md` → 1,1,1 |
| 8 | Gate 5 (`unreadable — re-run…`) | "0 and 0 on PR B"; no one-line rule for the wrapped FAQ | b716087 0,0; the FAQ phrase is now said to be kept on one line (GATE CONTRACT 10) | `grep -c` → 0,0 |
| 9 | Gate 5 (`protected — re-run…`) | "1 on PR B" | 1 at b716087 | `grep -c` → 1 |
| 10 | Gate 5 (CHANGELOG count) | before 0 (c1810a5) → after ≥ 8 "one per scope item 1–8" | before 6 at b716087 (`### Changed` 3, `### Fixed` 3; next heading `## [0.2.13] - 2026-09-30`, which the end pattern matches) → after ≥ 10 (6 + items 3, 4, 6 + this leaf's Documentation entry) | `sed -n '/^## \[Unreleased\]/,/^## \[0\.2\.13\]/p' CHANGELOG.md \| grep -c '^- \*\*'` → 6; `grep -n '^## ' CHANGELOG.md` → 8 Unreleased, 71 0.2.13; the three sibling leaves each plan ≥ 1 `### Added` entry |
| 11 | Gate 6 (P/S0/L six files) | "PR B (from its spec)" | b716087 (measured, release): `P 2`, `P device 2`, `S0 2`, `S0 device 2`, `L 0`, `L device 0`, `denial text in 0 of 6 files, remedy in 0`, with the stderr lines | gate run verbatim (dir created with `mkdir -p`, no `rm`) |
| 12 | Gate 7 (S `--job`) | "PR B (from its spec) the `ps` run exit 2 with the skip line" | b716087: no `exit` line, `grep` exits 2 on missing `target/pc15.err`; the S wrapper runs `--job` as a command (`_: --job: command not found`, rc 127); without `--job`, `S -- hmn ps` exits 2 with the `NoGpuSource` skip line | gate run verbatim; `sandbox.sh S -- "$H" ps` → 2 |
| 13 | Gate 8 (D) | 40a701e only | b716087: `exit 1`, 25 rows, 24 nameless (hmn's own the named one), stderr `hmn: 25 GPU processes found (927 MiB committed total; 24 protected — re-run outside the sandbox).` | gate run verbatim → `exit 1`; `ps --json` under D → one `"name":"hmn"` |
| 14 | Gate 9 (seam) | 40a701e only | b716087: `hmn= probe=`, exit 1 (`count_denied.py`, `--job` absent) | gate run verbatim |
| 15 | Gate 10 (compare guard) | "from compare.py's spec"; "PR B against itself prints the same five values" | b716087: the gate itself → `rc=2` (`compare.py: no such directory: …/captures/final` + usage); c1810a5 vs pr_b without `--spill-map` → rc 1, `none: DIFF`, `ps.stdout: pid N: SPILL '?' != 'n/a'` lines; pr_b vs pr_b → `rc=0`, 2, 49 and 47, 2 | the three `compare.py` runs below |
| 16 | Gate 11 (App Sandbox) command | ran `target/appsb/release/hmn` in place, stderr to `target/a.err` | copies the signed binary into `mktemp -d /private/tmp/hmn-appsb.XXXXXX` and runs it there (SIGTRAP 133 in place under `~/Documents`, per part1_close's `app_sandbox/README.md`) | edited gate run verbatim on b716087 → `1`, then the three lines in row 17 |
| 17 | Gate 11 before | "PR B (from its spec)" | b716087 (signed copy, `/private/tmp`): `1`, `[ps] 2 unreadable=0 skipped=1`, `[ps --device 0] 2 unreadable=0 skipped=0`, `[watch 1 …] 2 unreadable=0 skipped=0` | as row 16; a cwd under `~/Documents` is fine when the binary is under `/private/tmp` (`--version` rc 0, `ps` rc 2) |
| 18 | Gate 12 (Claude Code) | "the same proof greps as part1_close's"; before "files exist, pr_c.md absent, count 2" | part1_close's greps plus an `outcome:` line, which part1_close's records lack; b716087: c1810a5.md and pr_b.md each give 1 ×5, `envsandbox:` 1 (`SANDBOX_RUNTIME=1`), `outcome:` 0; commits c1810a5ff8…, 4a849e00…; distinct count 2; probe directory named | proof-grep loop over both files → `1 1 1 1 1 1 0` each; gate run verbatim → `2` |
| 19 | Gate 13 (tests) macos_smoke | before 2 / "3 on PR B" → after 3, not guarded | labelled `(guard)`: 3 at b716087 (the three names) → 3. b716087 → after was 3 → 3 with no label (GATE CONTRACT 9) | `cargo test --locked --all-features --test macos_smoke -- --ignored` → 3 passed; `--list` → 3 |
| 20 | Gate 13 Rosetta and macos_sandbox | "the PR B head's count plus 39" | 95 at b716087 (native and Rosetta) + 39 = 134; `macos_sandbox` target absent at b716087 | `cargo test --locked --all-features [--target x86_64-apple-darwin] --lib` → 95 passed each; `--list` → 95; no `tests/macos_sandbox.rs` |
| 21 | Gate 13 `ps --device 1` / `watch 0` | "`1` and `0` on PR B" | b716087 (release): `1` and `0`; `ps --device 1` exit 2 with `hmn: ps failed to query device 1: device index 1 out of range (have 1 devices)` | gate's `bash -c` run verbatim |
| 22 | Gates 14, 15, 16 (CI record, R01) | "on PR B" ×6 | b716087: CI file absent (`grep` exit 2, no stdout); PR B's record gives `4`,`0`,`1` (run 37210951264, four `branch=expected`); ✅ rows 7 (items 1,2,5,7,8,9,10); fresh-eyes 1 (line 347); `pending, PR C` 4; When-it-bites pattern 0 over 7 rows (PR A's Claude Code row included); `only together with the denial line` 1 (Verification, wrapped over 3 lines) | `grep -c` on R01 and the CI files; `grep -n '^## '` on R01 |
| 23 | Gate 17 (`__reports__`) | 18 (c1810a5), 6 (40a701e), "more on the PR C base" | 145 files (20 `field_check_v0213`, 125 `v0214_part1`) and 31 lines at b716087 | `git ls-files __reports__ \| wc -l` → 145; `git grep -n … \| wc -l` → 31 |
| 24 | Steps 1–5 prose | Step 1: FAQ "on PR B"; Fallback cell's backticks not mentioned; main.rs bullet "ps_watch_unreadable wrote"; CHANGELOG "one per scope item". Step 2: App Sandbox run in place; `enabled-by` options lack the Desktop toggle; `sandbox_sysctl_probe.py` with no path; Rosetta "PR B head's count". Step 3: "keep the rows" (8 named); L-residual row kept verbatim. Step 4: `git diff v0214-part1...HEAD`. Step 5: 40a701e list only; Deliverables lack CHANGELOG.md, kinfo.rs | Step 1: b716087 values, backticks kept, main.rs bullet is part1_close's (ends `see README Limitations, item 9.`), six CHANGELOG entries under Changed/Fixed. Step 2: copy to `/private/tmp` and run there; `enabled-by` also names the Claude Desktop "Local sandbox" toggle (what pr_b.md records); probe path given; Rosetta 95 + 39. Step 3: all twelve PR B rows listed; drop only `as README Limitations item 9 states` from the L-residual row. Step 4: `git diff b716087...HEAD`. Step 5: b716087's 31 lines listed; Deliverables add `CHANGELOG.md` and `src/gpu/kinfo.rs` | R01 lines 305–320 (12 PR B rows, 315 says "as README Limitations item 9 states"); main.rs:178; README:523; pr_b.md `enabled-by:` line; the `git grep` output below |

## Spec defects not fixed

1. **The +39 Rosetta delta depends on two other leaves.** 134 = 95 + 32 (kinfo_enumeration) + 9
   (gpu_process_listing) − 2 (bridge). If another sweeper changes kinfo_enumeration's 32 or
   gpu_process_listing's 9, this gate goes stale. ps_watch_unreadable adds no lib tests to the sum
   (its tests are bin-side), so if it does add some, the sum is wrong. Proposed fix: after the
   A5 sweep the coordinator recomputes the delta from the final leaves. Not chased here.
2. **Cargo.toml `exclude = ["__reports__/"]` (ef24b99) after `git rm -r __reports__`: keep it.**
   Measured on b716087: `cargo package --list --locked --allow-dirty` lists 0 `__reports__` paths
   with the directory present and gives no warning either way. With the directory removed it is a
   no-op (80 lines, 0 warnings). Reasons to keep it:
   - this project re-adds `__reports__/` during development (f3c6010 dropped it, v0.2.14 brought it back), and the exclusion protects any release cut while it exists;
   - dropping it is a `build` change outside PR C's docs scope;
   - the gate's `git grep` does not cover `Cargo.toml`.

   Note that R01:401 ("Since PR B, `Cargo.toml` excludes `__reports__/`…") holds the string, so
   Step 5 must reword it whichever way the coordinator decides. The leaf's Step 5 rule ("every
   mention") already covers this. Left to the coordinator, as asked.
3. **Step 3 item 1 versus item 3.** Item 1 creates the four PR C rows "in the table's columns: …
   the evidence path", and item 3 says "quote the results here, do not link to them". Every PR B
   row on b716087 links into `__reports__/`, and Step 5 then rewrites those links. Proposed fix:
   the PR C rows' Evidence cell names the record file plainly (Step 5 turns it into name + SHA),
   or quotes the result. The coordinator should pick one wording.
4. **`sandbox.sh` option position (for kinfo_enumeration's sweeper, not this leaf).** On b716087
   the parser takes options only before `PROFILE` (usage `sandbox.sh [--print-profile] PROFILE
   [--] CMD`). kinfo_enumeration places `--job` after `PROFILE` (`S --job --`), and this leaf's
   S and seam gates use that form. This is consistent with kinfo_enumeration's spec, but it means
   the extension has to parse a second option slot. Recorded so the two leaves stay in step.
5. **The Claude Code `hmn-commit:` of PR B is 4a849e0, not b716087** (PR B's head when the
   record was taken, before the squash/merge). The gate's "three distinct commits" still holds.
   The commit SHA in a record is history, so this is not a defect, but R01's Verification text
   should not name 4a849e0 as "PR B".

No gate on b716087 has before = after without a `(guard)` label, now that edit 19 labels
macos_smoke. The CHANGELOG gate was weak (≥ 8 would pass with 2 new entries over b716087's 6)
and is tightened to ≥ 10 (edit 10). The coordinator may revert that to "≥ 9" if a Documentation
entry is not wanted.

Leftover: `SCRATCH/appsb_part2_close/` (the signed App-Sandboxed b716087 `hmn` and two stderr
files). The `/private/tmp/hmn-appsb.*` directory the gate created has been removed, and so has
the measurement worktree.

## Commands run on b716087 (from the measurement worktree; CLI gates under `bash -c`)

```
git -C …/v0214-part2 worktree add --detach $SCRATCH/m_part2_close b716087
rustup check | grep stable                           # up to date: 1.99.0
cargo build --release --locked
sandbox-exec -p '(version 1)(allow default)' /usr/bin/true   # rc 0
# static befores (bash -c, /usr/bin/grep)
tr '\n' ' ' < README.md | grep -o 'measures what is permitted and counts the rest' | wc -l   # 0
grep -cF 'the sandbox decides' README.md; grep -cF 'denies only' README.md; grep -c 'refuses .proc_listpids' README.md  # 1 1 1
for f in docs/FAQ.md src/bin/hmn/main.rs src/lib.rs src/gpu/mod.rs; do tr … | grep -o … | wc -l; done   # 0 0 0 0
grep -c 'Limitations, item 9' docs/FAQ.md src/bin/hmn/main.rs                 # 2 3
grep -c 'enumeration, names and lookups try libproc first' README.md src/lib.rs  # 0 0
grep -c 'process lookups try libproc first' README.md src/lib.rs              # 1 1
grep -c 'not retried' README.md src/lib.rs                                    # 0 0
grep -c -i 'cross-user' README.md docs/FAQ.md src/lib.rs src/gpu/mod.rs src/gpu/metal.rs src/bin/hmn/main.rs src/bin/hmn/ps.rs  # 1 1 0 0 0 0 0
grep -c 'always succeed' README.md src/lib.rs                                 # 0 0
grep -c 'sudo hmn ps' README.md docs/FAQ.md src/bin/hmn/main.rs src/bin/hmn/ps.rs   # 0 0 0 0
grep -c 'sudo' README.md                                                      # 1
grep -c 'outside the sandbox' README.md docs/FAQ.md docs/tutorials/watching-a-running-job.md  # 1 1 1
grep -c 'unreadable — re-run outside the sandbox' README.md docs/FAQ.md       # 0 0
grep -c 'protected — re-run outside the sandbox' README.md                    # 1
sed -n '/^## \[Unreleased\]/,/^## \[0\.2\.13\]/p' CHANGELOG.md | grep -c '^- \*\*'   # 6
grep -c '^| [0-9]* | .*| ✅' docs/roadmap-v0.2.14.md                           # 7
grep -c 'To be filled in\|To be run with fresh eyes' docs/roadmap-v0.2.14.md  # 1
grep -c 'pending, PR C' docs/roadmap-v0.2.14.md                               # 4
grep -c -E '\| (listed|partial|blind|unchanged)[^|]* \|$' docs/roadmap-v0.2.14.md   # 0
grep -c 'only together with the denial line' docs/roadmap-v0.2.14.md          # 1
git ls-files __reports__ | wc -l                                              # 145
git grep -n "__reports__" -- docs README.md ROADMAP.md CONVENTIONS.md CHANGELOG.md src | wc -l   # 31
  # CHANGELOG.md:19,27,33,45  ROADMAP.md:218  dogfooding:387,388  src/gpu/kinfo.rs:32
  # docs/roadmap-v0.2.14.md:68,95,108,137,230,275,300,301,305,309-312,314-320,398,401,410
grep -n '__reports__' Cargo.toml                                              # 13: exclude = ["__reports__/"]
# harness
python3 harness/compare.py pr_b pr_b                  # none: … (rows compared: 49, spill cells mapped: 0, allowlisted lines: 0); C: … 47 …; rc 0
python3 harness/compare.py c1810a5 pr_b               # none: DIFF, ps.stdout: pid N: SPILL '?' != 'n/a' …; rc 1
python3 harness/compare.py c1810a5 pr_b --spill-map   # none/C identical (47, 50, 1); rc 0
python3 harness/compare.py pr_b __reports__/v0214_part2/captures/final   # no such directory + usage; rc 2
find pr_b c1810a5 -name '*.exit' | wc -l              # 36
# claude_code_sandbox proof greps on c1810a5.md / pr_b.md     # 1 1 1 1 1 1 0 each; distinct hmn-commit 2
# CLI befores (release)
P/S0/L six-file gate                                  # P 2, P device 2, S0 2, S0 device 2, L 0, L device 0; 0 of 6, remedy in 0
S --job gate                                          # `_: --job: command not found`, wrapper rc 127, grep: no such file
sandbox.sh S -- hmn ps                                # 2, NoGpuSource skip line
D gate                                                # exit 1 (25 rows, 1 named "hmn", 24 protected — re-run outside the sandbox)
seam gate                                             # hmn= probe=, rc 1
hmn ps --device 1 | grep -c 'device index 1 out of range (have 1 devices)'   # 1 (exit 2)
hmn watch 0 --duration 2s --interval 1s | grep -c 'names no running process' # 0
P: hmn watch 1 --duration 1s --interval 1s            # 2, NoGpuSource
# tests
cargo test --locked --all-features --test macos_smoke -- --ignored            # 3 passed (--list 3)
cargo test --locked --all-features --lib                                      # 95 passed (--list 95)
cargo test --locked --all-features --target x86_64-apple-darwin --lib         # 95 passed (--list 95)
# App Sandbox (recipe of field_check_v0213/evidence/probes/appsandbox)
RUSTFLAGS="-C link-arg=-Wl,-sectcreate,__TEXT,__info_plist,$PL/Info.plist" cargo build --release --locked --target-dir target/appsb
codesign -s - -f --entitlements $PL/entitlements.plist target/appsb/release/hmn
edited gate (copy to mktemp -d /private/tmp/hmn-appsb.XXXXXX)              # 1; [ps] 2 0 1; [ps --device 0] 2 0 0; [watch 1 …] 2 0 0
# Cargo.toml exclude
cargo package --list --locked --allow-dirty | grep -c __reports__             # 0, no warnings
git rm -r -q __reports__ && cargo package --list --locked --allow-dirty       # 80 lines, 0 warnings; restored with git checkout
git -C …/v0214-part2 worktree remove --force $SCRATCH/m_part2_close
```
