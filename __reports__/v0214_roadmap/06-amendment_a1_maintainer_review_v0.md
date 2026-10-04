# Gap Analysis: v0214-sandbox — the maintainer's PR A review asks PR B for four things no leaf owns (A1)

## Problem Statement
PR A (mi-for-the-rust-of-us/hypomnesis#6) merged on 2026-10-04 as 1bb9b22. The maintainer's approving review added work for PR B. Two of its points are already covered: the rule that `ps_exit_status_is_1_when_nothing_is_listed_and_opt_in` accepts exit `2` only with the `(skipped)` line (`ps_failed_devices` Step 1, confirmed by its verifier), and adversarial review of the new `unsafe` in item 2 (`process_exists_kinfo` had its Sonnet verifier, PASS). Four points are owned by no leaf:

1. `exclude = ["__reports__/"]` in `Cargo.toml`, so the 18 report files do not ship in a release cut before the directory is dropped.
2. One sentence in R01 saying which remedy wording goes where: PR B keeps the `N protected` count with the macOS remedy, and PR C adds `N unreadable`.
3. A toolchain-freshness check in the gate set: confirm local `stable` is the current release before trusting a green run.
4. "Whenever convenient": `__reports__/field_check_v0213/00-findings_v0.md` links to the report's old file name.

## Evidence
Measured at bf450aa (`v0214-part1`, the five code leaves on 1bb9b22):
- `cargo package --list --locked | grep -c '^__reports__/'` → 18 (98 files in all, 22 under `src/`). `Cargo.toml` has no `include` or `exclude`.
- `grep -c 'PR B ships the macOS remedy on the existing' docs/roadmap-v0.2.14.md` → 0. R01 *Decisions taken* fixes `N unreadable — re-run outside the sandbox`, and only the *PR split* B row mentions `N protected — re-run outside the sandbox`.
- `grep -c 'rustup check' docs/roadmap-v0.2.14.md` → 0. The campaign README's gate set has no freshness check either. `rustup check` on 2026-10-04: `stable-aarch64-apple-darwin - up to date: 1.99.0`.
- `git grep -c cross-user-ledger` → `__reports__/field_check_v0213/00-findings_v0.md:1` (line 112). The file in `docs/dogfooding-feedbacks/` is `dogfooding-macos-sandbox-eperm-and-device-bounds.md`.

## Root Cause
The roadmap was written before the review. Points 1 to 3 are new requests, and point 4 is a stale link PR A did not touch.

## Impact Assessment
- Scope: `Cargo.toml` (one line); `docs/roadmap-v0.2.14.md` (*Decisions taken*, *Verification*, *At release*); one line of `__reports__/field_check_v0213/00-findings_v0.md`. There is no Rust code, so the crate's API and behaviour do not change.
- Dependencies: none on the five code leaves. part1_close (one level deeper) edits R01's *Verification* placeholder, *Scope* statuses and *When it bites* sentence. This leaf edits other lines of R01 and runs first, so the two never touch the same lines.
- Risk if not done: point 1 ships `__reports__/` in any release cut from PR B alone. Points 2 and 3 leave two maintainer requests unanswered in PR B.

## Proposed Solution
One amendment leaf, `part1/maintainer_review_a.md`, a sibling of the five code leaves, so BFS finishes it before `close/`. It has three steps: `build(crate)` for the exclude, `docs(roadmap)` for the remedy-split sentence and the freshness gate, and `docs(reports)` for the link. The coordinator also adds the freshness check to the campaign README's gate set and to the per-commit gate script used before every push. The leaf is mechanical, so it merges on the implementer's report with no verifier, like `metal_bounds_check` and `spill_cell_na`.

## Recommendations
1. Approve A1 and dispatch it before the draft push of PR B, so the push carries it.
2. Mention in PR B's body that the exclude and the R01 sentence answer the PR A review.
3. Coordinator adjudication recorded with A1, not a leaf: the `process_exists_kinfo` verifier found that a zombie (exited, not yet reaped) now reads `Some(true)` on macOS, where it read `Some(false)`, and that nothing documented it. The coordinator added one clause to the `process_exists` rustdoc and its CHANGELOG bullet at integration (bf450aa), since that leaf owns those lines under OWNERSHIP.

Approved by the user on 2026-10-04 ("New amendment leaf").
