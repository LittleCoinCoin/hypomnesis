# v0.2.14 part 1 (PR B): field check on the M3 Pro

Apple M3 Pro, macOS 26.6.2, 2026-10-04 (rows 7 and 8 retaken 2026-10-05). "Before" is
c1810a5 (the v0.2.13 tree the campaign measured), "after" is PR B: a build at 4a849e0 for
the CLI and test fixtures (see *History* below for that commit). CLI fixtures use default-feature release builds
(`cargo build --release --locked`), run from an unsandboxed `bash`, with the profiles of
[`harness/sandbox.sh`](harness/README.md). Process counts and sizes follow the live process
list, so they vary between rows and runs; no raw process list is committed.

## Fixtures

| # | Fixture | Before (c1810a5) | After (PR B) | Result | Evidence |
|:--|:--------|:-----------------|:-------------|:-------|:---------|
| 1 | `hmn ps --device 1`, unsandboxed | exit 2, `NoGpuSource` text naming NVML, DXGI, PDH and `nvidia-smi` | exit 2, `device index 1 out of range (have 1 devices)` | PASS | [`fixtures/cli.txt`](fixtures/cli.txt) |
| 2 | `hmn ps --device 1`, profile P | exit 2, the same `NoGpuSource` text | exit 2, `device index 1 out of range (have 1 devices)` | PASS | [`fixtures/cli.txt`](fixtures/cli.txt) |
| 3 | `hmn watch 0 --duration 2s --interval 1s` warns `names no running process` | exit 0, 1 warning line | exit 0, 0 warning lines | PASS | [`fixtures/cli.txt`](fixtures/cli.txt), [`pr_b/none/watch_0.stderr`](pr_b/none/watch_0.stderr) |
| 4 | `hmn ps`, profile P | exit 0, `hmn: 0 GPU processes found.`, header-only table | exit 2, `hmn: ps failed to query device 0: … (skipped)` and `hmn: ps: no device could be queried, so nothing could be listed`, no stdout | PASS | [`fixtures/cli.txt`](fixtures/cli.txt), [`pr_b/P/ps.stderr`](pr_b/P/ps.stderr) |
| 5 | `hmn ps --exit-status`, profile P | exit 1 | exit 2 | PASS | [`fixtures/cli.txt`](fixtures/cli.txt) |
| 6 | `hmn ps --json`, profile P | exit 0, `[]` (3 bytes) | exit 2, 0 bytes | PASS | [`fixtures/cli.txt`](fixtures/cli.txt) |
| 7 | unsandboxed output unchanged apart from `n/a` | (base capture) | `compare.py --spill-map`: `none: identical after normalisation (rows compared: 47, spill cells mapped: 50, allowlisted lines: 1)`; exit 1 without `--spill-map` | PASS | [`harness/README.md`](harness/README.md), [`c1810a5/none/`](c1810a5/none/), [`pr_b/none/`](pr_b/none/) |
| 8 | Codex-policy output unchanged apart from `n/a` | (base capture) | `compare.py --spill-map`: `C: identical after normalisation (rows compared: 47, spill cells mapped: 50, allowlisted lines: 1)` | PASS | [`harness/CODEX_PIN`](harness/CODEX_PIN), [`c1810a5/C/`](c1810a5/C/), [`pr_b/C/`](pr_b/C/) |
| 9 | `cargo test --locked --all-features --test macos_smoke -- --ignored` | 2 passed | 3 passed (adds `process_exists_under_sandbox_profiles`) | PASS | [`fixtures/tests.txt`](fixtures/tests.txt) |
| 10 | `kern_proc_pid_record_matches_the_kernel_for_this_process`, `--target x86_64-apple-darwin --lib` under Rosetta 2 | 0 such tests | `... ok`, 1 passed; the whole x86_64 lib: 94 passed | PASS | [`fixtures/tests.txt`](fixtures/tests.txt) |
| 11 | `cli_ps` test binary under profile P | exit 0, 2 passed, no `branch=` line | exit 0, 2 passed (1 ignored), `cli_ps: with --exit-status branch=skipped-device` and `cli_ps: without --exit-status branch=skipped-device` | PASS | [`fixtures/cli_ps_under_P.txt`](fixtures/cli_ps_under_P.txt) |
| 12 | `hmn ps`, profile D | exit 0, `20 GPU processes found (…; 19 protected — re-run elevated for names)` | exit 0, `20 GPU processes found (…; 19 protected — re-run outside the sandbox)` | PASS | [`fixtures/cli.txt`](fixtures/cli.txt) |
| 13 | Claude Code sandbox, c1810a5 binary | (no earlier measurement of this build in this harness) | lists normally: 18 GPU processes, `ps rc=0`; SPILL `?`; `watch 0` warns `names no running process` | PASS | [`claude_code_sandbox/c1810a5.md`](claude_code_sandbox/c1810a5.md) |
| 14 | Claude Code sandbox, PR B binary (never exit 0 with `0 GPU processes found`) | c1810a5: lists normally (row 13) | lists normally: 18 GPU processes, `ps rc=0`; SPILL `n/a`; no `watch 0` warning | PASS | [`claude_code_sandbox/pr_b.md`](claude_code_sandbox/pr_b.md) |
| 15 | App Sandbox `hmn ps` | exit 0, `hmn: 0 GPU processes found.` | exit 2, the `(skipped)` line and the closing line, no stdout | PASS | [`app_sandbox/README.md`](app_sandbox/README.md) |
| 16 | App Sandbox `hmn ps --device 0` | exit 2, `NoGpuSource` naming NVML, DXGI, PDH and `nvidia-smi` | exit 2, `NoGpuSource` naming Metal, NVML and `nvidia-smi` | PASS | [`app_sandbox/README.md`](app_sandbox/README.md) |
| 17 | `hmn ps`, profile L (the documented residual) | exit 0, `hmn: 0 GPU processes found.` | exit 0, `hmn: 0 GPU processes found.`, as README Limitations item 9 states | PASS | [`fixtures/cli.txt`](fixtures/cli.txt) |
| 18 | `cli_ps` acceptance branch on PR B's macos-latest CI | PR #5's run (36979948122): 0 `branch=` lines | run 37210951264 (PR #7, head fdf2f6c, 8/8 jobs green): 4 `branch=` lines, all `branch=expected` (with and without `--exit-status`, on `macos-latest / 1.88` and `macos-latest / stable`), none rejected | PASS | [`ci_macos_cli_ps.md`](ci_macos_cli_ps.md) |

Notes on the rows:

- (3): the same binary's committed capture agrees: `grep -c 'names no running process'`
  gives 1 for `c1810a5/none/watch_0.stderr` and 0 for `pr_b/none/watch_0.stderr`.
- (7), (8): captured with `capture.py` on 2026-10-05 from the c1810a5 release build and
  the PR B release build at the rewritten Step 3 commit. A second capture of the PR B
  binary, compared with the committed one without `--spill-map`, is identical for both
  policies (rows compared 47, spill cells mapped 0, allowlisted lines 0), so the
  normalisation is stable for one binary. With one NAME hash edited in a scratch copy,
  `compare.py` exits 1.
- (11): run with `--nocapture`, so the passing test's branch lines are printed.
- (18): `branch=expected` means the exit code matched the old rule: on GitHub's macOS VMs
  `hmn ps` could query the device, so the exit-2 branch did not fire there, and nothing was
  rejected. The VMs therefore do not exercise the skip path; row (11) does, under profile P.
- (13), (14): one sandboxed session, run by the maintainer. Each file carries
  `nested sandbox-exec rc=71`, `sandboxed: 1`, `envsandbox: SANDBOX_RUNTIME=1`,
  `claude-code: 2.1.273` and its own `hmn-commit:`. Claude Code's sandbox does not deny
  `process-info` to `hmn`, so neither build hits PR B's exit 2 there.
- (15), (16): the App-Sandboxed binaries ran from a copy outside `~/Documents`. Run in
  place under `~/Documents`, both builds die with `SIGTRAP` before `main`, even for
  `--version` (see the App Sandbox README).

## Residuals

- **Profile L.** A Seatbelt profile that denies only the per-process `ledger` read
  (`process-info-ledger`) still gets the old silent zero from PR B: `hmn ps` lists nothing
  and exits `0`. README Limitations item 9 says so. PR C closes it.
- **`__reports__/` in the tree.** PR B alone would ship `__reports__/v0214_part1/`
  (harness, captures, these findings) in the repository; `Cargo.toml` already keeps
  `__reports__` out of the crates.io package. The reports are dropped by PR C, or by a
  follow-up before release if PR C does not ship.
- The macOS "re-run elevated" advice of the first draft is no longer a residual:
  `remedy_macos` replaced it with `re-run outside the sandbox` (row 12).

## History

- **The Claude Code captures' `hmn-commit`.** Both
  [`claude_code_sandbox/`](claude_code_sandbox/pr_b.md) files, and the CLI and test
  fixtures of this table, ran PR B at 4a849e0: part1_close Step 3 as it stood before
  amendment A3 rewrote the branch. Its source outside `__reports__/` matches the rewritten
  Step 3 commit apart from the Step 2 rustdoc and `--help` prose, so `hmn ps`'s behaviour
  is the same. The two capture files are left as recorded.
- **`remedy_macos`'s `Known residual:` gate.** Amendment A3 deleted the `Known residual:`
  paragraph from the `format_ps_summary_with` rustdoc (dev history in shipped rustdoc), so
  `grep -c 'Known residual:' src/bin/hmn/ps.rs` reads 0, where `remedy_macos`'s historical
  gate recorded 1. No PR C leaf cites that gate. The race it described, a process that exits
  between enumeration and its name lookup being counted as protected, is unchanged.
- **The harness scripts.** Amendment A3 ported `capture.sh` and `compare.sh` to
  `capture.py` and `compare.py`, dropped the live `rows=<count>` from the `ps_json`
  normalisation, and retook the `c1810a5/` and `pr_b/` captures with them (rows 7 and 8).
