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
| 7 | `PAGED` / `device` SPILL cells, `paged` / `shared_share` JSON, the device verdict on the summary line (request 2) | feature | ⬜ |
| 8 | `hypomnesis::process_exists` (observation 1) | feature | ⬜ |
| 9 | `hmn watch` warns about a nonexistent explicit PID (observation 1) | feature | ⬜ |
| 10 | `hmn watch` rows aligned under the header (observation 2) | fix | ⬜ |
| 11 | Limitations after the command list in `hmn --help` (observation 3) | docs | ⬜ |
| 12 | README, FAQ, tutorials, `CHANGELOG.md`, `ROADMAP.md` | docs | ⬜ |

---

## Verification

To be filled in as items land.

---

## Consistency pass

To be run with fresh eyes after the last commit.

---

## At release

- `Cargo.toml` bumped to `0.2.13`; this roadmap's status and the dogfooding report's `Status`
  flipped, per the dogfooding style guide.
- The README's "what's new" banner rotated: 🆕 `0.2.13`, `0.2.12` to 🚀, `0.2.10` dropped.

---

## References

- [`docs/dogfooding-feedbacks/dogfooding-spill-verdict-wording-and-ps-filters.md`](dogfooding-feedbacks/dogfooding-spill-verdict-wording-and-ps-filters.md)
  — the report this release implements.
- [`docs/dogfooding-feedbacks/dogfooding-spill-triage-watch-mode.md`](dogfooding-feedbacks/dogfooding-spill-triage-watch-mode.md)
  — Verdict 3, why the `hmn ps` mark means *paged*, not *cause*.
- [`docs/roadmap-v0.2.12.md`](roadmap-v0.2.12.md) — `hmn watch --filter`, whose semantics
  `hmn ps --filter` copies, and the deferral of the `ps` twin.
