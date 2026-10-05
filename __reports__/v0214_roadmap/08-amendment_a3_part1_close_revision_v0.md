# Gap Analysis: v0214-sandbox — part1_close revision after its verifier (A3)

## Problem Statement
part1_close's Sonnet verifier returned PASS WITH NOTES (tip 1147621, unpushed). The user reviewed the notes with the coordinator on 2026-10-05 and decided:

1. **Withdrawn:** `compare.sh` ignores rows that vanish. PR C's guards already require `rows compared: N ≥ 1` per policy, and comparing over PIDs present in both captures is the design for live data.
2. **Fix:** the committed baselines drift in one place. PR C (`kinfo_enumeration`'s guard, `part2_close` Step 2) compares a fresh capture against the committed `pr_b/`. The only live value not masked was `ps_json`'s `rows=<count>`; a later capture of the same binary showed `rows=19` against `rows=17` and a DIFF. `ps_json` is reduced to `keys=<sorted key names>`. The alternative considered was back-to-back captures of both binaries at PR C time; the user chose the one-line normalisation.
3. **Fix, minimally:** prose. "On macOS a name is `?` only when a sandbox withheld `proc_pidpath`" (FAQ, `ps.rs` rustdoc) contradicted the rustdoc's "Known residual" paragraph (a process that exited also reads `?`). That paragraph, mandated by `remedy_macos`, narrated development ("v0.2.13 had the same race… not new… deliberately unchanged here"). The user's rule for outgoing text applies: no meta-discourse, every text written as the final state; history goes to reports and the roadmap. So "only" is dropped, the "Known residual" paragraph is deleted, and in the README and `--help` Security note the macOS sentence moves after "None of these are intrinsically malicious…", whose antecedent it had displaced.
4. **Withdrawn:** App Sandbox evidence "prose only". The command outputs are committed (`app_sandbox/{c1810a5,pr_b}.txt`), and the decisive control is on record: the same source without the entitlement lists 19 processes, while with it `hmn ps` exits 2.
5. **Fix, mandatory:** `harness/codex.sb` is a verbatim copy of an Apache-2.0 file from openai/codex, which has a NOTICE ("OpenAI Codex / Copyright 2025 OpenAI"). `CODEX_PIN` gains a licence-and-attribution line, and the harness README states it. It cannot go in `codex.sb` itself: the pin's hash covers the upstream bytes.
6. **New, user decision:** `capture.sh` and `compare.sh` are this campaign's own scripts (upstream has no shell scripts; its only scripts are the PR A Python probes), and the maintainer works on Windows. They are ported to Python 3 (stdlib only): `capture.py`, `compare.py`. `sandbox.sh` stays shell: it only wraps `/usr/bin/sandbox-exec`, which exists only on macOS.

Raised by the verifier and recorded here, not fixed: committed name hashes are unsalted 8-hex SHA-256 prefixes, so common app names can be recovered by dictionary. The user did not take up the salted-hash option.

## Evidence
The verifier's report of 2026-10-05: mutations in scratch copies of `compare.sh`; a fresh capture against the committed `pr_b` giving `ps_json` DIFF; `sha256("WindowServer")` beginning `02d2ede4`. Coordinator reads: `git ls-tree 1bb9b22` shows no `.sh` upstream; openai/codex at 696b450 has `LICENSE` (Apache-2.0) and `NOTICE`; and only `remedy_macos.md` cites `Known residual`.

## Root Cause
The leaf specs prescribed the shell harness, the `rows=` field and the residual paragraph. The verifier read them against later use (PR C, a public PR, licences), which the specs had not.

## Impact Assessment
- Scope: part1_close's unpushed commits only, each fix folded into the step that owns the file. Step 2 also removes the "Known residual" paragraph that `remedy_macos`'s pushed commit added.
- Contracts: the harness entry points become `python3 …/capture.py` and `python3 …/compare.py`, with CLI, outputs and exit codes unchanged. The roadmap leaves that cite them (campaign README, `part1_close`, `kinfo_enumeration`, `ps_watch_unreadable`, `part2_close`) are renamed in the same roadmap commit. The historical verification reports keep the old names.
- Historical gates that no longer hold, by decision: `remedy_macos`'s `grep -c 'Known residual:' src/bin/hmn/ps.rs` = 1 (now 0).
- Evidence: both trees are recaptured with `capture.py`. The Claude Code capture's `hmn-commit` 4a849e0 names part1_close Step 3 before this revision rewrote it; the findings say so.

## Proposed Solution
The part1_close implementer revises its five commits (fixups folded by rebase), re-runs every gate of the leaf and the full gate set on each commit, and a Sonnet verifier checks the delta. No new node: A3 changes part1_close's deliverables.

## Recommendations
1. Approved by the user on 2026-10-05 ("go ahead, and port them to Python"; ps_json row count removed).
2. PR B's body describes the final state only. The review record goes in `__reports__/`.
