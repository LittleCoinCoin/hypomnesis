# Roadmap verification — v0214-sandbox, round 3

One focused Sonnet verifier (V-R3) attacked only the round-2 changes:

- **Result:** 0 blockers, 1 confirmed major, 5 confirmed minors, 4 PLAUSIBLE.
- **Seams:** all held. That covers the shared `error.rs` `mod tests`, `pub fn device_query_failure_line`, the 4 "pending, PR C" rows, plain mention plus SHA against the `git grep __reports__` = 0 gate, the Claude Code row, and the `skipped-device-nogpu` labels.
- **Clippy:** all seven fixer prototypes reproduce clean (stable 1.99, 1.88 and the Linux target). The one exception is the red Step 1 stub state, which is expected and never pushed.

The fixes below were small text edits, so the coordinator applied them directly. A final verifier
(V-R4) checks only these edits.

| ID | Finding | Fix applied by the coordinator |
|---|---|---|
| M1 (major) | `kinfo_enumeration` and `ps_watch_unreadable` pre-conditions quoted a `ps_failed_devices` gate in wording that is not verbatim. | Both now quote `"four commands, each written out in full"`, which `grep -F` matches in `ps_failed_devices.md` (1 hit). |
| m1 | `part2_close` still said docs_pr's *CI* bullet repeats "only together with the denial line". The coordinator's reword had removed it. | The gate's PR B before value is now 1, because the CI bullet avoids the phrase. Step 3 item 4 rewrites only the *Verification* sentence. |
| m2 | CI's 1.88 clippy/test leg was missing from `part2_close`'s gate-set pre-condition and from the R01 gate set docs_pr writes. | Added to both. R01's list gains five bullets. |
| m3 | The SHA guard would treat a 10-digit number such as `5947395182` as a SHA. | Each candidate must contain a hex letter (`grep "[a-f]"`), in both places the guard appears. |
| m4 | `ps_failed_devices` never said `let _ =` for the `writeln!` log line. | Added, with the reason: `unused_must_use` under `-D warnings`, and `expect` is denied. |
| m5 | README CLIPPY IN TESTS named four lints. | It now lists the denied `match_wildcard_for_single_variants` and `indexing_slicing`, the warned `as_conversions` and `cast_*` lints, and the idioms `.get(..)` and `try_from`. |
| p2 | `kinfo_enumeration` did not say how test PIDs are built. | `i32` literals, or `i32::try_from(std::process::id())` with an explicit arm, and never `as i32`. |
| p4 | `remedy_macos` quoted "the macOS remedy ships in PR B". | Aligned to docs_pr's token "ships with PR B". |
| p1 | The `pub(crate)` 0 → 0 guard could be tripped by a doc comment quoting the leaf's rationale. | REJECT: it is a guard, and an implementer copying that rationale into rustdoc would be caught by review anyway. Low risk, no change. |
| p3 | `part2_close` step 5 allows paths without a SHA after the final rebase. | REJECT as a roadmap change. If the maintainer squash-merges, branch SHAs become unreachable whatever we record. The PR C body will say which SHAs refer to the pushed branch, and the maintainer chooses the merge style. Recorded as a residual. |

## Round 4 (V-R4): the coordinator's edits

- **Result:** 0 confirmed blocker or major defects. Every edit is confirmed as described.
- **docs_pr tokens:** none of the 21 gate-grepped tokens is split across lines.
- **Simulation:** writing the edits into a scratch R01 moves every docs_pr gate count as stated.
- **SHA guard:** it flags an off-branch SHA and ignores `5947395182`, tested in a scratch repository.

V-R4 found five stale phrasings, all fixed:
- docs_pr's Deliverables said "three added commands"; it now says five.
- docs_pr's U3 bullet now names CI's 1.88 leg.
- part2_close's final gate set gains the two +1.88 commands.
- remedy_macos cites R01 §Decisions taken with its exact wording.
- docs_pr's CI bullet says "(the maintainer's request on issue #3)". It no longer uses the campaign label R03, which R01 does not define.

The roadmap is final: `dirtree-rdm validate` passes on all 18 files.
