# Audit brief: is each PR C addition worth keeping?

We are contributors to someone else's crate (hypomnesis, maintainer PCfVW). PR C is implemented on
branch `v0214-part2` (b716087..03bab46, unpushed). It came out much larger than expected:
`src/` + `tests/` +2,862/−320; `src/gpu/metal.rs` +982 (+261 code, +152 comments, +549 tests).
Implementing agents and review rounds added functions, seams, guards and tests that neither the
maintainer nor our own design asked for. **Your job: judge, item by item, whether each addition in
your area really improves the code and serves the goal better than what was agreed, and whether it
reads like the maintainer's own code.** Do not dismiss the agents' work by reflex, and do not keep
it by reflex: argue each item from the evidence.

## The yardsticks, in priority order
1. **What the maintainer asked**: issue mi-for-the-rust-of-us/hypomnesis#3, his comment
   https://github.com/mi-for-the-rust-of-us/hypomnesis/issues/3#issuecomment-5947395182
   (`gh api repos/mi-for-the-rust-of-us/hypomnesis/issues/comments/5947395182 --jq .body`), and his
   PR B review (`__reports__/v0214_part1/01-reviews_v0.md`, section "The maintainer's review"; and
   `gh pr view 7 --repo mi-for-the-rust-of-us/hypomnesis --comments`).
2. **What we designed with him**: R01, `docs/roadmap-v0.2.14.md`, sections *Design decisions taken
   before starting* and *Scope* (items 3, 4, 6).
3. **The maintainer's idiom**: `CONVENTIONS.md`, and above all how *he* writes code in this crate.
   Calibrate on code he authored, not on our PR B additions: e.g. `git log --format='%h %an %s'
   -- src/gpu/nvml.rs src/gpu/pdh.rs src/bin/hmn/ps.rs src/bin/hmn/watch.rs`, and read his
   functions and his `mod tests` (size of functions, how much is factored into pure helpers, how
   tests are written, how much rustdoc, how he handles FFI errno/return codes, e.g. NVML's named
   return-code consts and `NvmlSession`). Note how PR B's accepted additions (`kinfo.rs`,
   `classify_kern_proc_pid`, `decide_exists`) were shaped and that he accepted them.
4. **Why each addition was made**: the amendment records in the roadmap worktree
   `<worktrees>/v0214-roadmap/__reports__/v0214_roadmap/`:
   `10-amendment_a5_*` (trust_kinfo_listing), `11-amendment_a6_*` (classify_kinfo_all,
   fill_kinfo_all_with, comm_from_lookup, harness hardening), `12-amendment_a7_*`,
   `13-amendment_a8_*`, and the reviews under `11-a6_reviews/`, `12-a7_reviews/`, `13-a8_reviews/`.
   The leaves themselves: `__roadmap__/v0214-sandbox/part1/close/part2/**.md`.

## Where
- Branch to read: `<worktrees>/v0214-part2` (read-only;
  never edit, commit, build there). Base: b716087 (upstream main). `git diff b716087..HEAD -- <files>`.
- If you need to build or try a simplification, make your own detached worktree:
  `git -C <worktrees>/v0214-part2 worktree add --detach <SCRATCH>/<dir> HEAD`
  (`<SCRATCH>` = SCRATCH),
  local `cargo`, never set `CARGO_TARGET_DIR`; remove it when done. Trying is optional: this is an
  analysis, keep it proportionate.
- colgrep for search: `index_status`, `index_build` on the path you search, pass it on `search`.
  `grep` on a named file is fine.

## For each addition in your area, give
- **What**: the item (fn/type/test/seam), file:line, code lines and test lines it costs.
- **Asked?**: maintainer / R01 / the leaf (roadmap authoring) / added in review (which amendment).
- **Does it improve the code or the goal?** Concretely: what bug or silent wrong answer it prevents,
  how likely that is on real hardware, and whether a sentence in the docs or a stated residual would
  serve as well (PR B accepted "pinned by grep rather than a run" and documented residuals).
- **Idiom**: does it read like the maintainer's code, or like ceremony (abstraction for testability
  alone, closure injection, enums for test labels, tests that pin a seam rather than a behaviour)?
- **Verdict**: KEEP / SIMPLIFY (say how, and the rough line saving) / DROP (say what replaces it:
  nothing, a doc sentence, a residual).
Also flag the reverse: anything the goal needs that is missing or weaker than agreed.

## Report
Write `<SCRATCH>/<report file named in your prompt>`: a one-paragraph summary, then the table
(What | Asked? | Improves? | Idiom | Verdict | Δ lines), then a short list of the 3–5 changes you
would make first, with the rough line count after. Be concrete and brief; no ceremony. Final
message: the summary paragraph and the path.
