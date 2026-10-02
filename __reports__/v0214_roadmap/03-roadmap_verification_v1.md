# Roadmap verification — v0214-sandbox, round 1

Three Sonnet verifiers re-attacked all 11 leaves:

| Verifier | Scope | Confirmed blocker/major |
|---|---|---|
| V-AB1 | gate integrity for PR A and PR B; prototyped most PR B code and the whole harness | 0 blockers, 3 majors |
| V-C1 | gate integrity for PR C; measured profile semantics | 1 blocker, 7 majors |
| V-SEAM1 | coverage and seams; mapped the parallel PR B overlaps | 0 blockers, 3 majors |

Round 0 had about 40 blocker and major findings, so the round reduced them sharply.

Every recorded "before" value the verifiers re-ran matched, except the ones listed below. Every
round-0 row is APPLIED, apart from the PARTIALs fixed here: A-6/U3, C-1, C-4, G1, G8, and V-COV 12
and 14.

## The new fact behind the PR C blocker

**`hmn` holds a graphics footprint of its own.** V-C1 measured it:
- `hmn ps --json` lists hmn's own PID at 16384 bytes, unsandboxed and under profile D;
- `hmn watch --filter hmn` selects it;
- its own `ledger` read is allowed under P and S0.

After PR C, wherever hmn can read itself, `entries` holds hmn's row. Every gate that assumed "zero
rows" under P, S or S0 was wrong. The fact goes into the campaign README BASELINES.

## Adjudication

Every confirmed finding below is **FIX**. The PLAUSIBLE items are adjudicated individually at the
end.

### Global (coordinator, campaign README)

| ID | Source | Fix |
|---|---|---|
| R1-G1 | V-AB1 7, V-C1 20 | CLI gates run as `bash -c '…'`. The user's and the Bash tool's shell is zsh, where `MULTIOS` makes `cmd 2>&1 >/dev/null \| …` carry stdout too: 14 lines against bash's 1. |
| R1-G2 | V-AB1 8 | Two build modes, kept separate. Gate checks may use the `--all-features` debug binary and match a substring or prefix. Byte-comparison captures use `cargo build --release --locked` with default features. |
| R1-G3 | V-AB1 6 | OWNERSHIP says "in the same leaf". A docs step after the code step is fine, because the PR merges the leaf whole. |
| R1-G4 | V-C1 | BASELINES gains the self-row fact above. |
| R1-G5 | V-AB1 5 | Pre-conditions cite `ci.yml` or source files directly, never as "Phase 0 facts" the README does not carry. |
| R1-G6 | V-AB1 3, V-C1 5, U3 | Squash scope. Under U3 a pushed commit must be green, so a red step's tests must all turn green in the very next step. Otherwise the leaf states its squash group (e.g. "Steps 1–3 push as one commit"), or moves the tests to the step that greens them. |

### PR A — `docs_pr`

| ID | Source | Fix |
|---|---|---|
| R1-A1 | V-AB1 A-6 | U3 goes into R01's *Decisions taken* as a bullet, not only into Verification item 8b. Add a gate. |

### PR B

| ID | Leaf | Source | Fix |
|---|---|---|---|
| R1-B1 | `metal_bounds_check` | V-AB1 1 | The guard counts 2 (the attribute plus the test literal). Re-pin it to `grep -c '#\[cfg_attr(not(target_os = "macos"), error(' src/error.rs` = 1, or state 2. |
| R1-B2 | `process_exists_kinfo` | V-AB1 2, 3 | Stub parameters become `_pid`, with `#[allow(clippy::missing_const_for_fn)]` on the stubs. R1-G6: the Step 1 tests that only green at Step 3 move to Step 3, or Steps 1–3 are declared one pushed commit. The leaf's "clippy stays clean" claim is corrected. |
| R1-B3 | `remedy_macos` | V-AB1 4, V-SEAM1 m1, m2; V-C1 16 | `grep -c 're-run elevated' README.md` is 1 (README:285 quotes the Windows parenthetical). remedy_macos owns that line and appends the macOS clause to it, with a gate. The six-test guard counts 7, not 6 (its twin test shares the prefix), so re-pin it. Delete the stale "ps_watch_unreadable adds the `remedy` wrapper". **Seam with PR C (V-SEAM1 M1, V-C1 4):** state in remedy_text's doc that on macOS the purpose `"for names"` gives the bare `re-run outside the sandbox`. |
| R1-B4 | `ps_failed_devices` | V-SEAM1 14 | The `every device failed` rustdoc phrase is kept on one line in Step 3.2, or the gate greps a short token. |
| R1-B5 | `part1_close` | V-SEAM1 M2, M3, 12; V-AB1 9 | **M2:** item 9 says "elevation does not help", keeping `sudo` out of it, so README `sudo` → 1 holds. FAQ: the gate counts what `remedy_macos` wrote as well. **M3:** the "Fallback" cell (README and `lib.rs`) reads "no second backend; process lookups try libproc first, then `sysctl`". The "not retried" wording goes. **V-COV 12:** add a step item and evidence row that records which `cli_ps` acceptance branch fired on PR B's macos-latest CI, read from the CI log, citing `ps_failed_devices` gate 17. **V-AB1 9:** the `--device 1` before value cites the c1810a5 measurement in the campaign README BASELINES, not a capture that does not exist. |

### PR C

| ID | Leaf | Source | Fix |
|---|---|---|---|
| R1-C1 | `kinfo_enumeration` | V-C1 1 (blocker), 13, 14, 15, 18 | `legacy_entries` returns `None` iff `others_read == 0 && !denied_pids.is_empty()`, whatever `entries` holds. Its test uses `entries = [caller row]`. Gate 22 says the macOS `NoGpuSource` text is `(Metal, NVML, and nvidia-smi …)`, matching `metal_bounds_check`. Gate 20 says how its two JSON inputs are produced: raw `ps --json` from `$HMN_CAPTURE_RAW` or a direct run, not the normalised captures. The Step 4 check exercises the harness files it adds (h). `kern_proc_pid_comm`'s test also accepts the argv0 basename, as PR B's does. |
| R1-C2 | `gpu_process_listing` | V-C1 2, 8, 9, 10, 19 | Gate 17, S plain `ps`: exit 0, no `(skipped)` line, `grep -c 'GPU process'` ≥ 1. The self row is listed. Gate 14: `--nocapture 2>&1 \| grep -c '^LISTING '` = 5 and `grep -c skip` = 0, so a failed `sandbox_apply` cannot pass vacuously. Gate 21 checks cargo's exit status. Gate 20 prefixes `git rev-parse --verify v0214-part1`. The "no denial off macOS" claim names the CI job that runs it. |
| R1-C3 | `ps_watch_unreadable` | V-C1 2, 3, 4, 5, 10, 11, 12 | **Remedy seam:** `failure_detail` and `denied_pid_notices` pass the purpose `"for names"` (R1-B3). The pinned texts stay as written: `— re-run outside the sandbox` and `— re-run outside the sandbox (skipped)`. Gate 15's prose matches the self row: `N GPU process(es) found` plus the unreadable clause. Gate 23's S auto-select exits 0 and prints the `they are not followed` line once. The "found no GPU processes" message is exercised with `--min 1MiB`, which drops the 16 KiB self row. **U3/R1-G6:** Step 1 holds only the ps, format and cli_ps stubs and tests; Step 3 writes the watch tests and their implementation together. Gate 12 gets `git rev-parse --verify`. Gate 20 adds a `--list` count for the ignored test (G3). Add `grep -c explained_by_denial tests/cli_ps.rs` ≥ 3, which pins that `accept` uses it. |
| R1-C4 | `part2_close` | V-C1 6, 7, 17, 21; V-SEAM1 m3, m4, M3 | Gate 20 reuses part1_close gate 24's five proof greps: `nested sandbox-exec rc=[1-9]`, `sandboxed: [1-9]`, `hmn-commit:`, `envsandbox:` and `claude-code:`. Gate 22 maps `outcome: full` to `unchanged`. Gate 13 states the bold-led bullet style, citing CHANGELOG 0.2.13. Step 5 lists every `__reports__` link on the PR C base: R01's `claude_code_sandbox.md` and `pr_b.md` links, ROADMAP.md's notice link and any others. It gets a final `grep -rc` over `docs/ ROADMAP.md README.md` = 0. Add the gate `grep -cF 'denies only' README.md` 1 → 0, and a gate on the "Fallback" cell's PR C wording (M3). Gates 15–17 name their build line. |
| R1-C5 | `part1/close/part2/README.md` | V-C1 21 | Coordinator: cite part1_close's real gate number for the captures, not "baseline captures exist". |

### PLAUSIBLE items, adjudicated

- **The exited-process race** (V-SEAM1, V-AB1 (a)): a process that exits between enumeration and
  `proc_pidpath` gets `name: None`, so it is counted "protected", and unsandboxed macOS then
  advises "re-run outside the sandbox". This is the same race as v0.2.13's "re-run elevated", which
  is no worse, and PR C's four-outcome read drops PIDs that are gone at ledger time. **FIX, scoped:**
  `remedy_macos` documents it as a known residual in `format_ps_summary_with`'s doc. No behaviour
  change.
- **The placement of PR B's first push** (V-SEAM1): `ps_failed_devices` gate 10 and `part1_close`
  pre-condition 3 need a PR B CI run. **FIX:** the part1 README says PR B is pushed as a draft PR
  (with the user's yes) once the five code leaves merge, before `part1_close`, so CI evidence
  exists.
- **The FAQ anchor `On macOS`** (V-SEAM1): **FIX** — `kinfo_enumeration` step 3.2 anchors on a
  token unique to the NAME answer.
- **The hand-typed findings table** (V-AB1 (b)): **REJECT.** The table records field runs on real
  hardware. Each row cites an evidence file whose own greps are gated, so the `| PASS |` count is
  a completeness check, not proof.

## Next

Three fixers apply these items, one per PR level. The coordinator applies R1-G1–G6 and R1-C5. Round
2 re-runs gate integrity on every changed leaf, plus a seams pass on the three contracts touched
here: the remedy purpose string, the bridge, and the self row.
