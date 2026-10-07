# A5 sweep brief: bring one PR C leaf up to date with main (b716087)

You are one of four sweepers. Each takes one PR C leaf of the hypomnesis v0.2.14 roadmap and
checks every statement in it against upstream `main` at **b716087** ("Merge PR #7"), which is PR
C's base. The leaves were last revised before PR B's maintainer review (amendment A4), so they
drift from main. Assume they are wrong until measured. **Your job: measure, then edit that one
leaf's prose so it is true on b716087, then write a findings record.**

## Paths
- Roadmap worktree (branch `v0214-roadmap`, the spec):
  `<worktrees>/v0214-roadmap`
  - campaign: `__roadmap__/v0214-sandbox/` — read its `README.md` Gotchas first (GATE CONTRACT,
    LEAF GRAMMAR, SHELL, TWO BUILD MODES, SELF ROW, SANDBOX HARNESS, CLIPPY IN TESTS, RED COMMITS +
    SQUASH SCOPE, OWNERSHIP). Every rule there binds the edited leaf.
  - PR B leaves (all done, merged): `part1/*.md`, `part1/close/part1_close.md`. Their gate text is
    what PR C leaves quote in their pre-conditions.
  - PR C leaves: `part1/close/part2/kinfo_enumeration.md`, `.../api/gpu_process_listing.md`,
    `.../api/cli/ps_watch_unreadable.md`, `.../api/cli/close/part2_close.md`.
  - background: `__reports__/v0214_roadmap/06`–`09` (amendments A1–A4; A4 = PR B review).
- Main at b716087: read-only reference worktree
  `<worktrees>/v0214-part2` (branch `v0214-part2`).
  **Do not build or edit there.** For measurement, make your own detached worktree:
  `git -C <worktrees>/v0214-part2 worktree add --detach <SCRATCH>/m_<leaf> b716087`
  where `<SCRATCH>` is given in your prompt. Run `cargo` there, locally: never set
  `CARGO_TARGET_DIR`. Remove the worktree when done (`git worktree remove --force`).
- PR B's review record on main: `__reports__/v0214_part1/01-reviews_v0.md`. The harness on main:
  `__reports__/v0214_part1/harness/{sandbox.sh,capture.py,compare.py,codex.sb,CODEX_PIN,README.md}`;
  baselines `__reports__/v0214_part1/{c1810a5,pr_b}/`.

## Known drift (verified by the coordinator on b716087; fix wherever your leaf has it)
1. `kinfo.rs:55` already defines `pub(super) const ENOMEM: i32 = 12;` (un-cfg'd; used by
   `classify_kern_proc_pid`'s `rc != 0 && errno == ENOMEM` → `PidLookup::Unusable` arm). Reuse it;
   never add a second one. Its doc speaks of `KERN_PROC_PID` only.
2. `remedy_text` on main is `pub fn remedy_text(outside_sandbox: bool, purpose: RemedyPurpose) -> String`,
   `RemedyPurpose::{Names, Identify}`; `(true, Names)` is the bare `re-run outside the sandbox`.
   Every "for names" string call becomes `RemedyPurpose::Names`.
3. Test idiom: the maintainer's tests use `#[cfg(...)]` attributes (before `#[test]` on a fn, or on
   the statement inside, as `tests/smoke.rs:196-207` does), never `cfg!(...)` in an expression.
4. Base and branch: PR C's base is `PCfVW/main` = b716087, not `v0214-part1`'s head. Any gate that
   diffs against `v0214-part1` should diff against `b716087` (a SHA, so no ref can be missing).
5. Line numbers and before values: PR B added `kinfo.rs`, `RemedyPurpose`, `ENOMEM`→`Unusable`,
   `namelen` from `mib.len()`, the `cli_ps` sandbox test that fails (not skips) when it cannot
   sandbox, the DxgiQueryResult link fix, and more. Re-measure everything.

## What to check (every one, in your leaf)
- every symbol, signature, type, constant, file, module and attribute the leaf names or assumes
  (exists on b716087? same name, visibility, cfg, signature?);
- every quoted string (stderr lines, help text, doc sentences, test names) — exact bytes;
- every test name and every test count (`cargo test ... -- --list 2>/dev/null | grep -c ': test$'`),
  every `grep -c` before value, every cited line number (cite by symbol where you can; give a
  `~line` only beside a symbol);
- every pre-condition quoting a PR B gate: does the quoted text match the done PR B leaf, and is it
  true on b716087?
- every behavioural gate's **before**: where the leaf says "PR B (from its spec)" or predicts PR B,
  replace the prediction with the value **measured on b716087**, labelled `b716087 (measured)`.
  Keep the c1810a5/40a701e values (history), but the moving comparison is b716087 → after. If the
  b716087 before equals the after for a gate not labelled `(guard)`, that is a spec defect: say so
  in findings and either relabel it `(guard)` beside a moving gate or propose a moving gate.
- harness paths and options: which profiles `sandbox.sh` accepts on main, its option parser, what
  `capture.py`/`compare.py` print, the baseline file counts.
- the GATE CONTRACT (1)–(10) on every gate you touch: evidence named, before → after, `catches:`,
  expected-FAIL from an assertion, `--list` counts, guard rules, hard-wrapped files.
- tautology / unpassable: a gate that cannot fail, or cannot pass on main + the leaf's own change.

CLI gates run under `bash -c '...'` (zsh MULTIOS). `hmn` is `$PWD/target/release/hmn` built with
`cargo build --release --locked` (default features, no debug lines); a bare `hmn` is 0.2.3.
`/usr/bin/sandbox-exec` works in this session (nested rc 0). If it returns 71, stop and report.
`rustup check` must say stable up to date (1.99.0). colgrep: `index_status`, then `index_build` on
your measurement worktree's absolute path, then pass that path on every `search`; shell `grep -r`/`rg`
are blocked, but plain `grep` on a known file is fine.

## Editing rules
- Edit **only your leaf file**, and only its prose (the header Goal / Pre-conditions / Success
  Gates / References and each Step's fields). Never touch any `README.md`, any other leaf, or
  anything outside the roadmap worktree. Do not commit, do not push.
- Keep the LEAF GRAMMAR (one-line `- ⬜ ` gates, one-line `- [ ] ` pre-conditions, Consistency
  Checks ending `(expected: PASS)`/`(expected: FAIL)`, ≤ 5 steps, commit-type rules).
- After every edit run
  `bash <skill>/scripts/dirtree-rdm.sh validate <your leaf path>`
  and keep it exit 0.
- Change the minimum needed to make the leaf true; do not redesign it. Where main makes a design
  choice of the leaf wrong (not just stale), stop editing that point and report it as a spec defect
  with your evidence and a proposed fix; the coordinator decides.

## Report
Write `<SCRATCH>/a5_<leaf>.md`: a table `| # | Site (leaf section) | Was | Now | Evidence (command → output) |`
for every change you made, then a list of spec defects you did not fix (with evidence and a
proposed fix), then the commands you ran to measure b716087 befores. Your final message: the path
of that file, the count of edits, and the defects, briefly.
