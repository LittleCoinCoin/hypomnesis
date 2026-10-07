# Decision record: slimming PR C before it opens (pending A9)

## Why
PR C on `v0214-part2` (b716087..03bab46) delivers every behaviour agreed with the maintainer in issue #3 and R01. It came out at +2,862/−320 in `src/` and `tests/`, with `src/gpu/metal.rs` at +982. The user asked whether each addition made during implementation improves the code and serves the goal better than what was agreed, given that we contribute to someone else's crate and must stay in the maintainer's idiom (CONVENTIONS.md and his own code).

## Evidence
Three Sonnet audits, one per R01 item, read-only (`14-slim_audits/`). All three report that every agreed behaviour is delivered and correct, and that nothing is missing. The surplus is seams and the tests that pin them:

| Area | Added | Audit's estimate after |
|---|---|---|
| item 3: `metal.rs` + `kinfo.rs` | +982 / +134 | ~+520 / ~+100 |
| item 4: API and its tests | ~+665 | ~+430 |
| item 6: CLI and `tests/cli_ps.rs` | ~+920 | ~+370 |

The audits trace the surplus to two sources:
- **Roadmap authoring:** stub-first pure functions, and tests named up front.
- **Review rounds (A5–A8):** seams built to catch each surviving mutant, where PR B had accepted a residual.

## Decisions (user, 2026-10-07)
1. **Branch.** The cuts go on a new branch, `v0214-part2-slim`, cut from 03bab46. `v0214-part2` stays as the reference. The commits are rewritten in place, leaf by leaf, only once the slim branch is shown behaviour-identical, with the one intended change below excepted.
2. **`trust_kinfo_listing` stays.** It covers the case where a `kinfo_proc` of another size passes the whole-records check (1 time in 81 for 656-byte records), which would be a silent wrong listing. That is the class of bug the maintainer raised in his PR B review. R01's sentence "a larger record → `ENOMEM`" holds only for `KERN_PROC_PID` and is corrected.
3. **`hmn watch` aligns to R01.** The "N unreadable … they are not followed" line prints for `--follow-new` only. The leaf had extended it to plain auto-select. This is the only intended behaviour change.
4. **Process.** Three Sonnet slimmers work in parallel on disjoint files, keeping the interfaces between areas unchanged. One verifier then checks behavioural equivalence. The leaves' gate text is updated as amendment A9 once the slim branch verifies.
