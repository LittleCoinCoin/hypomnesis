# Roadmap verification — v0214-sandbox, round 0

Three adversarial Sonnet verifiers attacked the drafted roadmap (10 leaves):

- **V-COV:** coverage and ownership. It re-derived the work independently, then diffed the result
  against R08.
- **V-AB:** gate integrity for `docs_pr` and the five PR B leaves. It executed every check it could
  on c1810a5.
- **V-C:** gate integrity for the four PR C leaves. It executed every check it could, plus sandbox
  experiments.

Every recorded "before" value they could re-run matched. Live process-list counts drift, which is
expected. The docs and reports trees are identical between c1810a5 and 40a701e, so docs "before"
values hold on the rebased base.

The coordinator adjudicated every finding: **FIX** (with its owner), **MOOT** (made irrelevant by
another decision) or **REJECT** (with a reason). The user decided three of them (U1–U3).

## User decisions (2026-10-03)

- **U1 — PR B takes the macOS remedy.** A new parallel leaf, `part1/remedy_macos`, adds the remedy
  constant, `remedy_text` selected by `cfg!`, `format_ps_summary_with(.., outside_sandbox)` with
  the six pinned tests rewired (their literals unchanged), and `unresolved_growth_hint`. On macOS,
  PR B prints `N protected — re-run outside the sandbox`. `ps_watch_unreadable` extends these
  pieces and creates none of them. The pull-forward is recorded in R01 (`docs_pr`) and flagged in
  the PR B body.
- **U2 — `--exit-status` with a partial device failure.** When nothing is listed and at least one
  tried device failed, the exit is `2`. This goes into PR B's pure rule, and PR C's `ps_exit_code`
  takes `failed`. It is an R01 amendment: a fourth deliberate behaviour change, recorded by
  `docs_pr` and flagged in the PR B body.
- **U3 — red commits.** TDD keeps a red commit per step on the task branch. Before the coordinator
  pushes a PR branch, each red step commit is squashed into its green successor with
  `git reset --soft` and a recommit with `-F`, so every pushed commit is green. R01's "gate set on
  every commit" becomes "every pushed commit" (`docs_pr`).

## Global fixes (coordinator, campaign README)

| ID | Source | Fix |
|---|---|---|
| G1 | V-COV 7, V-AB 3–4 | U3 written into the Gotchas. Stub-step dead-code warnings (V-AB 3, 4) are **MOOT**: red commits never reach a pushed branch. A Step 1 stub must still compile, so the FAIL is an assertion. |
| G2 | V-AB 7 | An expected-FAIL check runs with `--no-fail-fast 2>&1`. It must show `grep -c 'panicked at'` ≥ N and `grep -c 'error\[E'` = 0. |
| G3 | V-AB 8, V-C 5–7 | An expected-PASS test-filter check first proves the filter matches: `cargo test … <filter> -- --list 2>/dev/null \| grep -c ': test$'` = N. That rejects "0 passed" vacuity and miscounted filters. |
| G4 | V-AB 14 | The contract allows an explicitly labelled `(guard)` whose before equals its after, provided it is paired with a moving gate. |
| G5 | V-AB 13 | The shared gate set adds `cargo check --locked --no-default-features` and `cargo check --locked --no-default-features --features nvml,dxgi,pdh` (from ci.yml). |
| G6 | V-C 9 | CLI gates use `$PWD/target/{debug,release}/hmn` and name their build command. A bare `hmn` resolves to `~/.cargo/bin/hmn` 0.2.3. |
| G7 | V-AB 2, V-COV 14 | A phrase gate on a hard-wrapped file (`main.rs` help, README, FAQ, tutorials) either greps a short token, or its step says the phrase is kept on one line. |
| G8 | V-AB 12, V-C 16 | A pre-condition quotes the cited gate's actual text, or cites "gate N of <leaf>". |
| G9 | V-AB 17, V-COV parallel | `ps.rs`, `format.rs` and `tests/macos_smoke.rs` get non-adjacent edits from parallel PR B leaves. The coordinator resolves the textual conflicts at rebase, and the OWNERSHIP line says so. |
| G10 | V-COV 15, V-C 13 | `kinfo.rs` items used only by `metal.rs` get `#[cfg(all(target_os = "macos", feature = "metal"))]`. Otherwise Linux clippy with `-D warnings` fails on them (executed by V-C). Owners: `process_exists_kinfo` for its items, `kinfo_enumeration` for `KERN_PROC_ALL` and `KinfoRead`. |
| G11 | V-COV 3–4, V-C 1, 8, 12 | **One sandbox harness, owned by `part1_close`:** `__reports__/v0214_part1/harness/{sandbox.sh,capture.sh,compare.sh}`. `sandbox.sh` defines these profiles: P (the report's); Q (P + deny `kern.proc`); S (P + allow `same-sandbox`, always run as `bash -c '"$@"; rc=$?; exit $rc' _ CMD` so a resident sibling exists); S0 (S, run directly: the command is alone, so it behaves like P); D (deny `process-info-pidinfo` except self, written out with `(version 1)(allow default)`); L (deny `process-info-ledger` only, R01's silent-skip path); and C (Codex: the base policy pinned to an openai/codex commit SHA, copied in with its source URL, plus `(allow file-read*)`). `compare.sh <base> <new> [--spill-map]` masks live counts and allowlists named differences. PR C reuses it. `kinfo_enumeration` step 4 *extends* the harness (the `gpujob.swift` fixture, `count_denied.py`) and creates no second `sandbox.sh`. `part2_close` calls `compare.sh` rather than a new `diff_baselines.sh`. |

## PR A — `docs_pr`

| ID | Source | Verdict |
|---|---|---|
| A-1 | V-AB F1 | FIX: delete "use `PCfVW/main` wherever a gate says `main`", and relabel the guard "measured at 40a701e". |
| A-2 | V-AB F2 | FIX: "20 → 23 (22 when step 2 is deferred under 2.7)". |
| A-3 | V-AB 1 | FIX: the Claude Code evidence must carry harness-provided proof as well as the sandbox check. Require a `claude-code: <version>` line, a user-confirmed `enabled-by:` line, and `env \| grep -i sandbox` output. Soften `catches:` to "an unsandboxed run or a retry outside the sandbox". A hand-written imitation profile is excluded by the step's procedure, not claimed as caught by the gate. |
| A-4 | V-AB 18 | FIX: the Intel guard also greps `Intel.*Rosetta\|Rosetta.*Intel`. The evidence file prints `sizeof(kinfo_proc)=648` only in its two output blocks. |
| A-5 | V-AB coord. | FIX: "the issue comment" means comment 5943801771, which is LittleCoinCoin's and carries the `blob/field-check-v0213-macos/…` links. |
| A-6 | U1, U2, U3, V-COV 16 | FIX: the step 1 R01 amendments. Under *Decisions taken*: the remedy pulled into PR B (U1), `--exit-status` partial failure → 2 as a fourth deliberate change (U2), and "gate set on every pushed commit" (U3). The PR split row for PR B gains `remedy_macos`. The CI note says PR B's acceptance is the skip line and PR C's is the `process list unreadable:` line. In *Verification*, the profile P line gives PR B's form (exit 2 plus the skip line) and PR C's form (exit 2 with the count and the remedy). |
| A-7 | V-COV 16 | REJECT, no owner needed: "future field checklists use a realistic dead PID" is guidance for the next checklist the maintainer writes. It already lives in R01's text and produces no artifact in this campaign. |

## PR B

| ID | Leaf | Source | Fix |
|---|---|---|---|
| B-1 | NEW `remedy_macos` | U1, V-COV adj. 1 | New parallel leaf. Its gates include the six pinned tests still passing off macOS (literals unchanged); a macOS-only test for `N protected — re-run outside the sandbox`; and `unresolved_growth_hint` text on macOS. CHANGELOG `### Changed`. It also updates the `main.rs` help that quotes the hint. |
| B-2 | `metal_bounds_check` | V-AB 7, 8, 11 | G2/G3 forms on its checks. "No test or doc quotes" becomes "no test quotes" (the dogfooding report quotes the text). |
| B-3 | `process_exists_kinfo` | V-AB 4, G10 | Keep the `kinfo.rs` `allow(dead_code)` until step 3 (consistent with 3.4). Gate the metal-only items with the macOS+metal `cfg`. Gate: `cargo clippy --locked --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings` exits 0. |
| B-4 | `ps_failed_devices` | U2, V-AB 16, V-COV 2, 12 | U2 rule in the pure fn and tests, with help and CHANGELOG to match. Gate 1 also pins `ALL_DEVICES_FAILED_LINE` by grep. The cli_ps acceptance test prints which branch fired (`eprintln!` captured with `--nocapture`), so PR B's CI log records it. Stale prose: README:258 "summary is always printed", the `ps.rs` "Always printed" doc, and README:201's exit codes. The skip-line format is the contract PR C keeps: `hmn: ps failed to query device N: <err> (skipped)`. |
| B-5 | `spill_cell_na` | V-AB 2 | The tutorial sentence is rewritten to contain `` `n/a` on Linux and macOS `` on one line (G7). The same applies to `main.rs`, FAQ and README. |
| B-6 | `part1_close` | V-AB 5, 6, 9, 10, 15; V-COV 3, 4, 8, 9, 13, 14 | Owns the G11 harness. Gate 21 counts `*.exit` only under `pr_b/` and `c1810a5/`, with one saved run each. The Claude Code re-run uses docs_pr's proof shape (A-3) for both binaries. The step 4 check counts `\| PASS \|` rows equal to the fixture count and `\| FAIL \|` = 0. `compare.sh` prints the number of `?`→`n/a` rows (≥ 1 unsandboxed) and allowlists the `watch 0` warning line and the per-PID `n/a`. R01 gets a PR B column or sentence for the Claude Code row, listed in Deliverables. The canonical statement (iii) is qualified to "a sandbox that refuses `proc_listpids`", and the profile-L residual (exit 0, `0 found` until PR C) is stated. The `--help` Security note (`main.rs` ~150) and its FAQ twin (~289) are added to steps 1–2, with a gate on the phrase. It is the only owner of the FAQ self-denying-profile line. Captures are normalised; no raw `ps --json` process lists are committed. `macos_smoke --ignored` → 3. The Codex policy copy is pinned (G11). The PR-B-alone residual is recorded: `__reports__/` is dropped by PR C, or by a follow-up if PR C does not ship. |

## PR C

| ID | Leaf | Source | Fix |
|---|---|---|---|
| C-1 | all PR C | V-C 1 (blocker) | S means the bash-wrapper form (G11), which reads 1 or more; S0 behaves like P. Every "S without a job" gate in `gpu_process_listing` and `ps_watch_unreadable` is rewritten. S0 → `ProcessListDenied`, exit 2 with the skip line carrying `process list unreadable:`. S → Ok, exit 0, the unreadable count. The before values are re-stated for PR B (exit 2 plus the skip line). |
| C-2 | `kinfo_enumeration` | V-C 5, 8, 9, 10, 16; V-COV 11–13 | G3 filter counts (`footprint_from_errno` matches 4; add the `footprint_unavailable` filter). The step 4 and gate 7 checks require `test -x $PWD/target/release/hmn`, a row count ≥ 1, and a `"name":"` count equal to the row count. Add a pure `libproc_outcome(written: i32, errno: Option<i32>) -> LibprocPids` with tests, so that ESRCH and ENOMEM never fall back and an empty list is `Failed`. Add a named single-PID comm lookup `kern_proc_pid_comm(pid) -> Option<Vec<u8>>`, built on `kern_proc_pid_raw` and `parse_kinfo_records`, with a test. Add gates for `legacy_entries` and for the P bridge (`ps --device 0` exit 2). `EPERM` lives in `kinfo.rs` beside `kinfo::ESRCH`. Remove PR B's `allow(dead_code)` note on the reader. Profile D is written out in full. `≥ 800` becomes a `count_denied.py` comparison. Gates 8 and 9 re-base their before values on the PR B tree, where `cross-user` is already 0; replace them with a PR C-specific token. Gate: `CONVENTIONS.md` `KERN_PROC_ALL` 0 → 1. The FAQ/`snapshot.rs` sentence "a macOS name is `?` only when `proc_pidpath` is withheld" becomes "only when both `proc_pidpath` and `KERN_PROC_PID` are refused". Uses the G11 harness. |
| C-3 | `gpu_process_listing` | V-C 1, 6, 14, 15, 16; V-COV 13 | Add gates: S with job → Ok with a non-empty `denied_pids`; S0 → `ProcessListDenied`. The `process_list_denied` filter matches 5 tests (G3). Add the guard `git diff v0214-part1 -- src/gpu/{nvml,pdh,nvidia_smi,dxgi,proc_name}.rs \| wc -l` = 0. The smoke grep becomes ≥ 1 per file. Add a positive `use hypomnesis::GpuProcessListing;` doctest beside the `compile_fail`. A unit case for a self-only read → `ProcessListDenied`, documented as "a sandboxed caller's own row is not returned on its own". Remove the phantom `# Limitations` reference. |
| C-4 | `ps_watch_unreadable` | V-C 1, 7, 11; V-COV 1, 10; U1, U2 | Pre-condition: `remedy_macos` gates met. It extends `REMEDY_OUTSIDE_SANDBOX`, `remedy_text`, `format_ps_summary_with` and `unresolved_growth_hint` and re-creates none of them. Skip line: `failure_detail(..)` becomes the `err` passed to `device_query_failure_line(idx, &detail, true)`. The gates say: starts with the prefix, contains `process list unreadable: ` and ` — re-run outside the sandbox`, ends with ` (skipped)`. `explained_by_denial` = `skipped_device_line && contains("process list unreadable:")`. A gate runs PR B's ignored test with `--include-ignored`. `ps_exit_code` takes `failed` (U2). The step 1 compile sites are named: the `SummaryNotes` literals at `ps.rs` ~1106/~1242/~1255 (`..Default::default()`) and both `missing_pid_notices` calls in the `test-helpers` test (`watch.rs` ~2152–2166, as rewritten by 587a6d5; pass `&[]`). G3 on `remedy_text`. Add a profile-L gate: exit 2 with `process list unreadable:`. It owns rewriting the `format_ps_summary` doc sentence about macOS `?` names. |
| C-5 | `part2_close` | V-C 2, 3, 4, 8, 12; V-COV 5, 6, 13, 14 | `macos_smoke --ignored` → 3. `cross-user`: README 1 and FAQ 1 as PR B leaves them, src files 0. README pointer sites say "see Limitations item 9" without repeating the headline, so the headline grep on README = 1. Add a step item that puts `outside the sandbox` in the FAQ. Step 2 calls G11's `compare.sh` (exit 0, empty diff, ≥ N PIDs compared). Step 3 gets a non-tautological check, since "To be filled in" is already 0 after `part1_close`; for example, every *When it bites* row carries a PR C result. The self-denying FAQ line is removed from its step (owned by `part1_close`). Grep the headline phrase on README only. Run the profile-L and S0 rows. |

## Rejected or moot

- V-AB 3, V-AB 4 (stub dead code on red commits): **MOOT** under U3/G1, with B-3 keeping the allows
  consistent anyway.
- V-COV 13, the `CONVENTIONS.md` row being cosmetic: kept anyway, because R06 lists the macOS calls
  by name. Cost is one line.
- A-7: rejected as above.

## Next

Three Sonnet fixers apply these items, one per PR level, so no two touch the same file. The
gate-integrity verifier then re-runs on every changed leaf, and the round repeats until no
confirmed finding remains (round 1 report: `03-roadmap_verification_v1.md`).
