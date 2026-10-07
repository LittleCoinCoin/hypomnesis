# Amendment A9: PR C slimmed to R01's shape

## Problem Statement
PR C delivered every agreed behaviour, but the review rounds (A5–A8) and the leaves' stub-first test lists had added seams and the tests that pin them: `src/` + `tests/` +2,862/−320, `metal.rs` +982. Three read-only audits (`14-slim_audits/audit_item{3,4,6}.md`) judged each addition against three references: the maintainer's comment on #3, R01's design, and the maintainer's own idiom. All three found the agreed behaviours delivered and about half the added lines to be surplus. The user's decisions are in `14-slim_decision_v0.md`.

## What changed
- **Slim branch first.** The cuts were made on `v0214-part2-slim`, by three Sonnet slimmers in parallel on disjoint files, with the interfaces between areas held fixed.
- **Equivalence (`14-slim_audits/eq_check.md`).** The slim branch was then checked against `v0214-part2` in one boot. The check covered 22 commands under eight Seatbelt profiles: 528 cells of exit code, stdout and stderr. It added `compare.py` and a rustdoc-JSON diff of the 486 crate items. Verdict: EQUIVALENT. The only difference is the intended one: `hmn watch` prints "N unreadable … they are not followed" for `--follow-new` only, as R01 states, plus its `--help` sentence.
- **Kept on the user's decision:** `trust_kinfo_listing`, the third layout guard. R01's sentence now says three guards stand and why a probe-sized `KERN_PROC_ALL` buffer needs the third.
- **Added back:** one assertion. `failure_detail` adds no remedy to `NoGpuSource`, the error a macos-latest VM gives. It is proven against its mutant.
- **Folded in place.** The cuts were folded into the leaf commits that own each file. `v0214-part2` is 6103aac..c026a93, ten commits on b716087. Its tree equals the slim tree, except for three records under `__reports__/v0214_part2/` that now name the final tests and residuals. The full version stays at `v0214-part2-full` (03bab46).
- **Size.** `src/` + `tests/` is +1,591/−317, and `metal.rs` +527. The lib has 118 tests, against 146 before; the `hmn` bin has 256, against 280.
- **The leaves.** Every gate the slimming superseded keeps its text and gains an "A9 (slimming):" note with the measured count and the test that pins the behaviour. Behaviours left without a test are stated residuals instead. `part2_close`'s expectations are updated: 118 lib tests, and 4 ignored `macos_smoke` tests now that `tests/macos_sandbox.rs` is folded into that file.

## Gates
Every one of the ten commits passes the gate set on its own (run per commit in a detached worktree):
- `cargo fmt --check`;
- clippy with `-D warnings`: default features, `--all-features`, and `--all-features --target x86_64-unknown-linux-gnu`;
- `cargo test --locked --all-features`;
- `cargo doc` with `-D warnings`;
- `cargo +1.88` check, clippy and test;
- `cargo check --no-default-features`, and the same with `nvml,dxgi,pdh`;
- `cargo test --no-default-features --lib --no-run`;
- `cargo test --target x86_64-apple-darwin --lib` under Rosetta.

## Residuals, carried to the record and the PR body
No test pins these:
- the libproc-first order;
- the retry bound;
- the count saturating at `u32::MAX`;
- `kern_proc_pid_comm`'s classify step;
- the watch "not followed" gate and the "found no GPU processes" count, which the profile-S runs cover;
- `accept`'s labels, which the CI log covers;
- `GpuProcessListing` staying `#[non_exhaustive]`.

Untested in both versions: the call-site wiring the equivalence check names.
