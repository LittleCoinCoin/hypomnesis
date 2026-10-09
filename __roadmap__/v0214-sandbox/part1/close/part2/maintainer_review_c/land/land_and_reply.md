# Autosquash, per-commit gates, hash repoint, push, reply

**Goal**: Fold review_code's and review_docs' `fixup!` commits into PR C's 18 commits. Every commit passes the per-commit gates, and the "at `6e7cf099e8`" mentions name the new consistency-pass commit. Then, with the user's yes at each outward step: force-push, CI green, and a table-led reply on the PR.

**Pre-conditions**:
- [ ] review_code and review_docs `done`, and review_code's adversarial verifier answered: its findings fixed by the same implementer, or recorded as residuals.
- [ ] Coordinator integration review recorded:
  - `Failed` and the PID 0 filter feed `decide_listing` the counts it expects: the caller is in neither `others_read` nor `denied`, PID 0 is in neither, and a deduplicated `denied_pids` gives the `ProcessListDenied` count;
  - the CHANGELOG and R01 match the final behaviour.

**Success Gates**:
- ⬜ [run] **Autosquash.** `GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash b716087` on `v0214-part2`, with the docs branch rebased onto the code branch first: 18 commits, no `fixup!` subject, and the subjects identical to those of `b716087..1c97d88`, in order (`diff <(git log --format=%s b716087..1c97d88) <(git log --format=%s b716087..HEAD)` is empty).
- ⬜ [run] **Per-commit gates.** `git rebase --exec "$GATES" b716087` exits 0, with the same `$GATES` as review_code. Every commit passes fmt, clippy (native and `x86_64-unknown-linux-gnu`) and the full test suite.
- ⬜ [run] **Full gate set at the head.**
  - The campaign gate set: `rustup check` up to date; `cargo doc -D warnings`; `cargo +1.88` check, clippy and test; the `--no-default-features` checks; the lib tests under Rosetta.
  - The Codex profile gate from review_code, on the head.
- ⬜ [static] **Hash repoint, last rewrite.**
  - `H=$(git rev-parse --short=10 <drop>~1)`, where `<drop>` is found by subject (`chore(repo): drop __reports__/ from the working tree`).
  - `git log -1 --format=%s $H` reads "docs(roadmap): record the consistency pass and mark scope items 3, 4, 6 and 11 done".
  - `git ls-tree -r --name-only $H -- __reports__/field_check_v0213 | wc -l` = 20, the same as at 6e7cf099e8.
  - `git grep -c 6e7cf099e8 HEAD` — before: 15 lines → after: 0.
  - `git grep -n "$H" HEAD | wc -l` = 15, and `git log -1 --format=%B <drop>` names `$H`, not `6e7cf099e8`.
  - The width stays 10 characters, so no wrapped line reflows.
- ⬜ [run] **Range-diff.** `git range-diff 1c97d88...HEAD` shows only the hunks review_code and review_docs list, plus the hash repoint.
- ⬜ [run] **Push and CI** (the user's yes first). Force-push with `git push --force-with-lease=v0214-part2:1c97d88 LittleCoinCoin v0214-part2`. Then `gh pr checks 8 --repo mi-for-the-rust-of-us/hypomnesis` shows every job green, including both macos-latest jobs.
- ⬜ [static] **Reply** (the user's yes first). A table-led comment on PR 8, one row per requested change and nit: what was done, and in which commit.
  - It says, in one line each: `Failed` covers the `rc == 0` anomalies too; `list_pids()` was not used, and why; the mentions number 15; their new hash.
  - The user re-requests review from the PR's Reviewers sidebar, since the fork token cannot.

**References**: the A4 reply (issuecomment-6006287998) for the reply's shape; campaign README Gotchas for the gate set.

## Step 1: Integrate and gate
**Goal**: One linear branch of 18 green commits carrying every A10 fix.
**Implementation Logic**:
1. Rebase `task/a10_docs` onto `task/a10_code`; the two touch disjoint files.
2. Fast-forward a scratch copy of `v0214-part2` to that tip.
3. Autosquash onto b716087, resolving conflicts to the final text, then run `--exec "$GATES"`.
4. Run the full gate set at the head. If anything is red, the same implementer fixes it (SendMessage).
**Deliverables**: `v0214-part2` rewritten locally (not pushed); no new product commit, and the Commit field is the roadmap record
**Consistency Checks**: `bash -c 'test "$(git rev-list --count b716087..v0214-part2)" = 18 && test "$(git log --format=%s b716087..v0214-part2 | grep -c "^fixup!")" = 0'` (expected: PASS)
**Commit**: `docs(roadmap): record the A10 integration of PR C`

## Step 2: Repoint the mentions
**Goal**: The 15 mentions and the drop commit's message name the new consistency-pass commit.
**Implementation Logic**:
1. Compute `H`.
2. `GIT_SEQUENCE_EDITOR` turns `pick <drop>` into `edit`, then run `git rebase -i $H`. At the stop:
   - `git grep -l 6e7cf099e8 | xargs sed -i '' "s/6e7cf099e8/$H/g"`;
   - write the amended message to a file with `git log -1 --format=%B | sed "s/6e7cf099e8/$H/"`;
   - `git commit -a --amend -F <file>`;
   - `git rebase --continue`.
3. Re-run the gates on the two replayed commits.
4. Any later rewrite at or before `H` means redoing this step.
**Deliverables**: the drop commit (amended, which the Commit field names) and 1c97d88's successor, replayed
**Consistency Checks**: `bash -c 'test -z "$(git grep 6e7cf099e8 HEAD)"'` (expected: PASS)
**Commit**: `chore(repo): drop __reports__/ from the working tree`

## Step 3: Push, CI, reply
**Goal**: The maintainer sees the requested changes, each in the commit it belongs to.
**Implementation Logic**:
1. Show the user the range-diff summary and the prose diff (CHANGELOG, R01, rustdoc), and ask before force-pushing.
2. After CI is green, draft the reply table and ask before posting it.
3. Record the Progress rows, mark the A10 nodes `done` in one batch, and push the roadmap.
4. Update memory, and append to the `sj-hypomnesis-pr8` reminder's scheduling log.
**Deliverables**: PR 8 head force-pushed; the reply comment; the roadmap Progress rows
**Consistency Checks**: `bash -c 'gh pr checks 8 --repo mi-for-the-rust-of-us/hypomnesis'` (expected: PASS)
**Commit**: `docs(roadmap): mark maintainer_review_c done — PR C's review answered`
