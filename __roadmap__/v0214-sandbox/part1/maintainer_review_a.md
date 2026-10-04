# A1: PR A review asks — package exclude, remedy split, toolchain freshness

**Goal**: Do what the maintainer's PR A approval asks of PR B that no leaf owns: keep `__reports__/` out of the crates.io package, state in R01 which remedy wording each PR ships, add a toolchain-freshness check to R01's gate set, and fix the stale report link in the v0.2.13 findings.
**Pre-conditions**:
- [ ] Amendment A1 approved by the user (2026-10-04, "New amendment leaf"), source `__reports__/v0214_roadmap/06-amendment_a1_maintainer_review_v0.md`
- [ ] Branch `v0214-part1` holds the five code leaves (bf450aa, on upstream main 1bb9b22, the PR A merge); this leaf branches from it
**Success Gates**:
- ⬜ [run] `bash -c 'cargo package --list --locked 2>/dev/null | grep -c "^__reports__/"'` — before (measured at bf450aa; Cargo.toml is unchanged since 1bb9b22): 18 → after: 0; and `bash -c 'cargo package --list --locked 2>/dev/null | wc -l | tr -d " "'` — before: 98 → after: 80, while `bash -c 'cargo package --list --locked 2>/dev/null | grep -c "^src/"'` — (guard) before: 22 → after: 22; catches: the key put under a table other than `[package]` (count stays 18), an `include` list that also drops sources or the README (total below 80, or `src/` below 22), and a pattern that misses nested report files (count above 0)
- ⬜ [static] `grep -c '^exclude = \["__reports__/"\]$' Cargo.toml` — before: 0 → after: 1 (the maintainer's exact text, on one line); catches: an `exclude` written as another glob that a later `__reports__/` subtree escapes
- ⬜ [static] `grep -c 'PR B ships the macOS remedy on the existing' docs/roadmap-v0.2.14.md` — before: 0 → after: 1 (one sentence in *Decisions taken*'s "Remedy wording fixed" bullet, that phrase on one line); catches: the remedy split left implicit, which the maintainer asked to have stated
- ⬜ [static] `grep -c 'rustup check' docs/roadmap-v0.2.14.md` — before: 0 → after: 1 (the first bullet of *Verification*'s gate-set list, which names the command once); catches: the freshness check added to the campaign's private gate set but not to the plan the maintainer reads
- ⬜ [static] `grep -c 'cross-user-ledger' __reports__/field_check_v0213/00-findings_v0.md` — before: 1 (line 112; `git grep -c cross-user-ledger` lists this file alone in the tree) → after: 0, and `grep -c 'dogfooding-macos-sandbox-eperm-and-device-bounds.md' __reports__/field_check_v0213/00-findings_v0.md` — before: 0 → after: 1; catches: a link that still points at the report's old file name
**References**: [PR A review](https://github.com/mi-for-the-rust-of-us/hypomnesis/pull/6) — the maintainer's approving review of 2026-10-04, "For PR B" and "Whenever convenient"; [R01 §Decisions taken](../../../docs/roadmap-v0.2.14.md) — "Remedy wording fixed"; [R01 §Verification](../../../docs/roadmap-v0.2.14.md) — the gate set; [R01 §At release](../../../docs/roadmap-v0.2.14.md) — the `__reports__/` drop

## Step 1: Keep __reports__ out of the crates.io package
**Goal**: Add the maintainer's `exclude`, so a release cut before `__reports__/` is dropped does not ship it.
**Implementation Logic**:
1. `Cargo.toml`, `[package]` table: add the line `exclude = ["__reports__/"]` after `categories`, exactly as the maintainer wrote it. `Cargo.toml` has no `include` and no `exclude` today, so this is the only package-content rule. Each PR can ship alone (R01 *PR split*), and part1_close adds `__reports__/v0214_part1/`, so the rule matters from PR B on.
2. `docs/roadmap-v0.2.14.md`, *At release*, the bullet "`__reports__/` dropped from the tree before the merge": add one sentence saying that `Cargo.toml` excludes `__reports__/` from the package since PR B, so a release cut before the drop does not ship it.
3. No CHANGELOG line: what a package contains is a release matter, and R01's *At release* records it.
4. Check with `cargo package --list --locked` on the committed tree (it needs no network).
**Deliverables**: `Cargo.toml` (`[package]` `exclude`); `docs/roadmap-v0.2.14.md` (*At release* sentence)
**Consistency Checks**: `bash -c 'test "$(cargo package --list --locked 2>/dev/null | grep -c "^__reports__/")" = 0 && test "$(cargo package --list --locked 2>/dev/null | grep -c "^src/")" = 22 && test "$(cargo package --list --locked 2>/dev/null | wc -l | tr -d " ")" = 80'` (expected: PASS)
**Commit**: `build(crate): keep __reports__ out of the crates.io package`

## Step 2: State the remedy split and the toolchain-freshness gate in R01
**Goal**: Make R01 say which remedy wording each PR ships, and put the freshness check first in its gate set.
**Implementation Logic**:
1. *Decisions taken*, the bullet "**Remedy wording fixed:** `N unreadable — re-run outside the sandbox` …": add one sentence, with the phrase `PR B ships the macOS remedy on the existing` on one line: "PR B ships the macOS remedy on the existing `N protected` count (`N protected — re-run outside the sandbox`); PR C adds the `N unreadable` count, which carries the same remedy." *PR split*'s B row implies this; the maintainer asked for it stated once.
2. *Verification*, the list after "Gate set on every pushed commit": add as its first bullet that `rustup check` must report `stable` up to date, so the local toolchain is the current release before a green run is trusted, because Rust 1.99.0's `assert_is_empty` lint reached CI before the local `stable` did (`587a6d5`). The command exits 0 whether or not an update exists, so the check is the `up to date` text on its `stable` line, not its exit status; say so in the same bullet without naming the command a second time (the gate counts it once in R01).
3. *Decisions taken*, the bullet "**Every pushed commit passes the gate set.**": its last sentence lists what extends *Verification*'s set; add "and the toolchain-freshness check".
4. Do not touch *Verification*'s "To be filled in as items land" line, the *Scope* statuses or the *When it bites* table: they are part1_close's.
**Deliverables**: `docs/roadmap-v0.2.14.md` (*Decisions taken*: the remedy-split sentence and the gate-set bullet; *Verification*: the freshness bullet)
**Consistency Checks**: `bash -c 'test "$(grep -c "PR B ships the macOS remedy on the existing" docs/roadmap-v0.2.14.md)" = 1 && test "$(grep -c "rustup check" docs/roadmap-v0.2.14.md)" = 1 && test "$(grep -c "To be filled in as items land" docs/roadmap-v0.2.14.md)" = 1'` (expected: PASS)
**Commit**: `docs(roadmap): state the remedy split and the toolchain-freshness gate`

## Step 3: Point the v0.2.13 findings at the report's current file name
**Goal**: Fix the one link that still names the dogfooding report's old file.
**Implementation Logic**:
1. `__reports__/field_check_v0213/00-findings_v0.md`, line 112: the link text and target `dogfooding-macos-cross-user-ledger-and-device-bounds.md` become `dogfooding-macos-sandbox-eperm-and-device-bounds.md`, the file in `docs/dogfooding-feedbacks/` (the relative path `../../docs/dogfooding-feedbacks/` is unchanged). The findings file is otherwise a historical record: change nothing else in it.
**Deliverables**: `__reports__/field_check_v0213/00-findings_v0.md` (line 112 link)
**Consistency Checks**: `bash -c 'test "$(grep -c "cross-user-ledger" __reports__/field_check_v0213/00-findings_v0.md)" = 0 && test -f docs/dogfooding-feedbacks/dogfooding-macos-sandbox-eperm-and-device-bounds.md && test "$(grep -c "dogfooding-macos-sandbox-eperm-and-device-bounds.md" __reports__/field_check_v0213/00-findings_v0.md)" = 1'` (expected: PASS)
**Commit**: `docs(reports): point the v0.2.13 findings at the report's current name`
