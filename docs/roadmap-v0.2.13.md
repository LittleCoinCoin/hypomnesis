# `hypomnesis` v0.2.13 — roadmap

> *Stop `hmn watch` from reporting a spill check it never ran, and let `hmn ps` say who is being
> paged.*

**Status: in progress.** Not pushed; no version bump until release.

---

## Why v0.2.13 (and not v0.3.0)

Every item is additive under the crate's own rule (`ROADMAP.md`, Principle 2: *"New variants and
fields land in patch releases. Type-shape changes … are minor bumps, never patches."*):

- one new public function, `hypomnesis::process_exists`; no existing item changes shape;
- new, off-by-default `hmn ps` flags (`--filter`, `--exit-status`, a repeatable `--pid`);
- two new `hmn ps --json` fields (`paged`, `shared_share`) beside the unchanged `spilling`;
- wording changes to human-readable output: the `hmn watch` summary on platforms where spill is
  not measurable, the `hmn ps` SPILL column and summary line, the `hmn watch` column alignment,
  and the order of the top-level `hmn --help`.

Two behaviour changes are deliberate, and each turns a silent wrong answer into a stated one:
`hmn ps --device <N>` with `N` out of range now exits `2` with an error instead of `0` with an
empty table, and the `hmn watch` text summary no longer says `no spill observed` for a run it
could not measure.

---

## Origin — a report from a rented Linux box and a Windows desktop

[`docs/dogfooding-feedbacks/dogfooding-spill-verdict-wording-and-ps-filters.md`](dogfooding-feedbacks/dogfooding-spill-verdict-wording-and-ps-filters.md)
(askesis `canvas`, 2026-09-28, revised 2026-09-30). On an RTX 5090 Linux rental, v0.2.12's
`hmn watch --filter` selected the trainer first time. But its text summary ended with
`episodes 0 — no spill observed` although the JSON said `"measurable": false`. On a Windows
RTX 5060 Ti, `hmn ps` flagged a real spill, but marked `SPILL` on all 25 rows, `hmn.exe` at 0 MiB
included. Requests, in the report's order: qualify the `watch` summary; say which process is
being paged; `hmn ps --filter`; an opt-in exit status for "nothing matched"; `--device` out of
range as an error; a repeatable `--pid`. Smaller observations: a nonexistent explicit PID is
watched silently; `watch`'s rows don't line up under its header; the top-level `--help` buries
the command list.

Checking the report against the code found request 1 to be a bug present since v0.2.6:
`format_watch_summary_text` prints the full spill block whenever a tracker exists and never
checks `SpillReport::measurable`, while `hmn spill` does. The one test named for the unmeasurable
case only covered "no tracker at all". It went unseen because under WSL2 `NVML` lists no
processes, so `hmn watch` exits `2` before printing any summary on the development machine.

---

## Design decisions taken before starting

- **The bug fix comes first,** in its own commit, ahead of the `hmn ps` work.
- **`hmn ps` marks the process being *paged*, not a *cause*.** The memory manager pages whatever
  it chooses; the process being paged need not be the one that pushed the card over (the
  report's revision, citing `dogfooding-spill-triage-watch-mode.md`'s third verdict). The SPILL
  column reads `PAGED` when the device is spilling and the row's SHARED is at least
  `DEFAULT_SHARED_GROWTH_BYTES` (256 MiB, the floor the spill condition itself uses), `device`
  on the device's other rows, and `no` / `?` as before. `hmn watch` keeps its column unchanged.
- **`--json` gains both `paged` and `shared_share`** beside the broadcast `spilling`, both
  `null` exactly when `spilling` is.
- **The summary clause needs no library change.** Free memory comes from `device_info` (already
  called for the device name); shared bytes and the paged count are summed over **all** of the
  device's rows, before any filter, so the device verdict does not depend on what is displayed.
- **`hmn ps --filter` has `hmn watch --filter`'s semantics,** and a row whose name cannot be
  resolved is counted on the summary line rather than dropped silently. Adding it is the fourth
  `hmn ps` filter, which `ROADMAP.md` named as the trigger for its `PsFilters` refactor; the
  refactor comes first, byte-identical.
- **A nonexistent explicit PID gets a warning, not a guess.** PID numbers carry no similarity, so
  "the nearest PID" would put a stranger's process under the user's name. A new library function
  answers "does this PID exist?"; `hmn watch` warns once and watches as before. Following a
  wrapper PID's GPU-holding descendants — the report's real launcher case — goes to
  `ROADMAP.md` as a candidate.
- **`hmn --help` keeps all its text,** with the per-platform Limitations moved after the command
  list.

---

## Scope

| # | Item | Kind | Status |
|---|---|---|---|
| 1 | `hmn watch` says "spill not measurable" instead of "no spill observed" (request 1) | **fix** | ✅ |
| 2 | `PsFilters`: one value for `hmn ps`'s filters | refactor | ✅ |
| 3 | `hmn ps --filter <PATTERN>` (request 3) | feature | ✅ |
| 4 | Repeatable `hmn ps --pid` (request 6) | feature | ✅ |
| 5 | `hmn ps --device` out of range exits `2` (request 5) | fix | ✅ |
| 6 | `hmn ps --exit-status` (request 4) | feature | ✅ |
| 7 | `PAGED` / `device` SPILL cells, `paged` / `shared_share` JSON, the device verdict on the summary line (request 2) | feature | ✅ |
| 8 | `hypomnesis::process_exists` (observation 1) | feature | ✅ |
| 9 | `hmn watch` warns about a nonexistent explicit PID (observation 1) | feature | ✅ |
| 10 | `hmn watch` rows aligned under the header (observation 2) | fix | ✅ |
| 11 | Limitations after the command list in `hmn --help` (observation 3) | docs | ✅ |
| 12 | README, FAQ, tutorials, `CHANGELOG.md`, `ROADMAP.md` | docs | ✅ |

---

## Deviations from the plan, and why

- **`--device` errors on any failed listing of the named device, not only out of range.** The
  report asked for the range case. A named device whose query fails for another reason produced the
  same empty table and exit `0`, the same "idle card" misreading; `hmn fits` already exits `2` on
  any `device_info` error. Without `--device`, a failing device is still skipped.
- **`matches_any` moved with `--filter` (item 3), not with the refactor (item 2),** and gained a
  sibling, `ps::filterable_name`: the one definition of "a name a pattern can match", which
  `watch::matchable_name` now builds on, so the two filters cannot disagree.
- **The `entry()` test fixture moved to `test_support.rs`** in item 2, once `ps.rs`'s tests needed
  it as well as `watch.rs`'s.
- **Docs landed with their features.** Each item updated the README, FAQ, tutorial, `--help` and
  `CHANGELOG.md` text it made stale; item 12 covers what remained (the README usage block and
  summary example, the FAQ's `hmn ps` SPILL entry, the spill tutorial's Step 3, `ROADMAP.md`).
- **`CHANGELOG.md`'s `[Unreleased]` sections now follow Keep a Changelog's order** (Added, Changed,
  Fixed, Security); the Security section was first.

## Verification

- Every commit passed the CI gate set on Windows: `fmt`, the two `--no-default-features` checks,
  clippy (default and all features, `-D warnings`), the tests, and `cargo doc` with `-D warnings`,
  also with `--document-private-items`. Tests: 324 → 355 passed.
- The macOS arm of `process_exists` and the Linux arm were checked with clippy for
  `aarch64-apple-darwin` and `x86_64-unknown-linux-gnu`, default and all features; the macOS one
  found an item-after-statement lint, fixed before commit.
- Tests pinned against the fix, not just alongside it: the `watch` summary test asserts the absence
  of `no spill observed` and `peak dedicated` for a non-measurable report; both alignment tests
  fail with the minimum widths removed.
- A new, non-ignored `tests/cli_ps.rs` runs the compiled binary: `--device 99` exits `2` with its
  reason and `--exit-status` exits `1` on an empty listing, on any machine, GPU or not.
- Live on the RTX 5060 Ti: `--filter DWM --filter code` lists both, case-insensitively, and echoes
  them; a blank pattern is rejected (exit `2`); `--pid A --pid B --pid A` lists two and echoes
  each once; `--device 3` exits `2` with `device index 3 out of range (have 1 devices)`;
  `--exit-status` exits `0` with a match and `1` without. With `tools/spillforge` forcing a real
  spill, `spillforge.exe` at 423 MiB shared read `PAGED`, every other row `device`; `--json`
  carried `"paged":true,"shared_share":0.9377`; the summary read `device 0 spilling: 1.0 GiB free,
  438 MiB shared, 1 process paged`. `hmn watch 999999 <own PID>` warned about 999999 only.
  `hmn watch --top 3` rows lined up under the header. Sorted, the old and new `hmn --help` are
  line-for-line equal, with the command list now above the Limitations.
- **Not seen live:** the Linux `hmn watch` summary. Under WSL2, `NVML` lists no processes, so
  `hmn watch` exits `2` before any summary; the fix is covered by the unit test on a
  non-measurable report, the exact shape a Linux run produces.

---

## Consistency pass

After the last item, a fresh agent that had not seen the work reviewed `c66cc87..HEAD` against
`CONVENTIONS.md`, for correctness and for doc drift. It found no functional bug; it re-ran clippy
for Windows, Linux and macOS and the tests, and checked the new behaviour live. Its findings,
fixed in one commit:

- **A test-module allow that suppressed nothing** (`src/gpu/mod.rs`, `unwrap_used`) — removed, as
  `CONVENTIONS.md`'s test-module rule requires.
- **Doc drift in the code:** `format_watch_rows_text` still said its widths came from each
  interval's own cells; `format_ps_summary` said "two appendices" and nested the unnamed count
  inside the committed total; the `ps.rs` module doc and a comment omitted `--filter`, the paged
  mark and `PsFilters`.
- **Doc drift in the docs:** the FAQ's `hmn watch` bullets lacked the nonexistent-PID warning and
  the "exit `0` also means not measured" caveat; the README said an absent PID renders `0 B`
  where its cells read `0 MiB`; the spill tutorial's rewritten Step 3 merged the two spill
  conditions into one; the README's JSON sentence was hard to parse.
- **One spelling:** `shared_share` now goes through `json_value_or_null`, whose doc names it, like
  every other optional JSON value; `resolved_name` moved to `ps.rs`, and `filterable_name` is now
  defined from it, so the unresolved-name brackets are listed once for both filters.
- **Idiom:** `process_exists`'s no-source arm uses the sibling dispatchers'
  `#[allow(unused_variables)]` rather than `let _ = pid` under `EXPLICIT`; the macOS `ESRCH` test
  is a comparison rather than a two-arm `match`; `paged_and_share` is a `const fn` (it compiles on
  MSRV 1.88, which clippy had not flagged).

After the fixes, the full gate set passed again on Windows (stable) and Ubuntu WSL2 (stable and
1.88), and clippy for macOS (stable and 1.88).

Left as they are, and why:

- **`PsFilters::new` and `Selection::new` each deduplicate PIDs in the same two lines.** A shared
  helper for two call sites was judged not worth it.

### Found after the consistency pass

- **Rows wider than a column still drifted, for that interval.** The review noted that item 10's
  minimum widths were the old header's (`NAME` 12, `PID` 6), kept so the header stayed
  byte-identical: a 13-character-plus name (`spillforge.exe`, `msedgewebview2.exe`) or a 7-digit
  Linux PID still pushed its row off the header. The maintainer chose to fix it rather than keep
  the header: `PID` now fits 7 digits, and `NAME` is sized once at attach to the longest watched
  name (12 at least), for the header and every interval's rows alike. No fixed width would do —
  Linux names are no longer cut at 15 bytes since v0.2.12 — and names are never truncated, since a
  name is what `--filter` matches. Only a longer name entering under `--follow-new` widens its
  column, for that interval. Live: `hmn watch <spillforge> <dwm>` lines up under a 14-wide `NAME`.
  Both alignment tests fail with the sizing disabled.

---

## At release

- `Cargo.toml` bumped to `0.2.13`; this roadmap's status and the dogfooding report's `Status`
  flipped, per the dogfooding style guide.
- The README's "what's new" banner rotated: 🆕 `0.2.13`, `0.2.12` to 🚀, `0.2.10` dropped.
- The `hmn watch` transcripts in `docs/tutorials/watching-a-running-job.md` and the README
  re-captured from the `0.2.13` build, so they show the aligned columns.

---

## References

- [`docs/dogfooding-feedbacks/dogfooding-spill-verdict-wording-and-ps-filters.md`](dogfooding-feedbacks/dogfooding-spill-verdict-wording-and-ps-filters.md)
  — the report this release implements.
- [`docs/dogfooding-feedbacks/dogfooding-spill-triage-watch-mode.md`](dogfooding-feedbacks/dogfooding-spill-triage-watch-mode.md)
  — Verdict 3, why the `hmn ps` mark means *paged*, not *cause*.
- [`docs/roadmap-v0.2.12.md`](roadmap-v0.2.12.md) — `hmn watch --filter`, whose semantics
  `hmn ps --filter` copies, and the deferral of the `ps` twin.
