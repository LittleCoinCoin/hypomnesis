# `hypomnesis` v0.2.12 — roadmap

> *Clean the base first, then teach `hmn watch` to follow a process by name.*

**Status: in progress.** Part 1 (audit remediation) ✅ done 2026-09-26 — nine items in ten commits,
then a `PDH` error-wording follow-up and a consistency pass; pushed at `ed2c2aa`, CI green. Part 2
(dogfooding features) ✅ implemented 2026-09-26, not yet pushed or released — see *At release*,
below.

---

## Why v0.2.12 (and not v0.3.0)

Both parts are patch-safe under the crate's own rule (`ROADMAP.md`, Principle 2: *"New variants and
fields land in patch releases. Type-shape changes … are minor bumps, never patches."*).

- **Part 1** is internal: eight behaviour-preserving refactors and one robustness fix, then two
  small visible changes, both additive under Principle 2. `Snapshot::ram_mb` became a `const fn`
  (every existing call still compiles), and the `PDH` error messages were reworded to
  `CONVENTIONS.md`'s form (text surfaced through `HypomnesisError`'s `Display`, which no code
  matched on). No public item was added or removed, and the `hmn` output contract — text and
  `--json` — stayed byte-identical, guarded by the existing exact-string tests plus the new
  key-parity test.
- **Part 2**'s requests are new, off-by-default `hmn watch` flags and a header line that changes only
  when one of them is passed. The one exception — a `{"kind":"start", ...}` record in the default
  `--json` stream — is still additive under Principle 2, but is the release's one deliberate
  wire-format change and will be scheduled as such (see part 2).

---

## Part 1 — Audit remediation

### Origin — a duplicate-code audit, read as a dogfooding report

Before implementing part 2, the crate was audited for duplicated code:
[`docs/audits/2026-09-26-duplicate-code-audit.md`](audits/2026-09-26-duplicate-code-audit.md). A
sliding-window hash detector found **no copy-paste at all** — zero duplicate groups at a 20- or
14-line window — but nine structural-duplication items. One had already caused a defect: v0.2.10's
`DXGI` skip-on-bad-adapter fix reached 2 of the 6 identical adapter walks. One left the
consumer-facing `SpillReport` JSON contract spelled four times with no test asserting the four
agree. The rest were maintenance load. Part 1 fixes all nine, one commit each, so that part 2 lands
on the cleaner base — in particular so the `start` record arrives as a *fifth* spelling of the JSON
contract only after a parity test guards the first four.

### Scope

| # | Item | Kind | Status |
|---|---|---|---|
| 1 | Four-way `SpillReport` JSON key-parity test | test only | ✅ |
| 2 | `spill_cell()`; `GpuDeviceInfo::name_suffix()` / `bytes_as_mib()` | refactor | ✅ |
| 3 | `DXGI` walks skip a bad adapter instead of aborting (4 remaining walks) | **fix** | ✅ |
| 4 | Docs-only: stale forward reference, `test-helpers` comment, `CHANGELOG` link references, `MemoryReport` MiB note, `tests/common` comment (the `spill_condition` roadmap rewording was withdrawn — see below) | docs | ✅ |
| 5 | `run_smi()` (`nvidia-smi`); `QueryGuard::open()` / `target_luid()` (`PDH`) | refactor | ✅ |
| 6 | One `SpillReport` JSON field writer replacing four spellings | refactor | ✅ |
| 7 | Split `src/bin/hmn.rs` per subcommand into `src/bin/hmn/` | refactor | ✅ |
| 8 | One `DXGI` adapter walker (visitor closure) for all six walks | refactor | ✅ |
| 9 | `NvmlSession` RAII guard; one column-table renderer | refactor | ✅ |

Design decisions taken before starting, and why:

- **`hmn.rs` split per subcommand** (`main.rs`, `summary.rs`, `ps.rs`, `spill.rs`, `watch.rs`,
  `fits.rs`, `format.rs`), following the file's existing section banners, rather than the audit's
  three-file sketch — which would have left `main.rs` at ~1,200 lines.
- **`DXGI` walker as a visitor closure, not an iterator.** `CONVENTIONS.md` Pattern 3: *"Do not store
  COM pointers across function boundaries."* A closure borrows each adapter inside the walker's own
  frame; an iterator would hand owned COM pointers to the caller.
- **Items 3 and 8 land separately, fix then refactor**, so the one behaviour change in part 1 is
  bisectable on its own and the refactor that follows can be checked as behaviour-preserving.

### Verification — each commit carried its own proof

Every commit passed the full gate set on Windows before landing: `cargo fmt --check`, `clippy
--all-targets -D warnings` with default features and with `--all-features` (so both
`debug-output` arms), `cargo check --no-default-features` and `--no-default-features --features
nvml,dxgi,pdh`, `cargo test --all-features`, and `cargo doc` under `-D warnings`. The test count
only grew: 289 → 295. On top of that, each change got the check that fits it, because "the tests
still pass" proves little about a refactor the tests were not written to watch:

| # | What had to be true | How it was shown |
|---|---|---|
| 1 | The new parity tests actually catch drift | Mutations — two middle keys swapped in one emitter, one key renamed in another — each failed a new test while every pre-existing test passed |
| 2, 5 | Output unchanged | Exact-string formatter tests unchanged; live smoke of every subcommand; for 5, ignored live tests on Windows and on Linux, where `process_gpu_info` goes through `nvidia-smi` and so through `run_smi` |
| 3 | The fix mirrors the reviewed v0.2.10 shape | Line-for-line with `enumerate_non_nvidia` / `device_count`; a real mid-walk adapter failure cannot be injected here (the reference machine has no iGPU) |
| 6 | The unified JSON writer is byte-identical | A throwaway probe wrote all four outputs before and after: 1,617 bytes, identical |
| 7 | The split moved code without changing it | Sorted list of all 173 bin test names identical before and after; `--help` for `hmn` and every subcommand byte-identical to a build of the previous commit |
| 8 | The walker preserves behaviour | `debug-output` `DXGI` traces from the 16 ignored live tests identical to a build of the previous commit; clippy across five `dxgi` feature combinations |
| 9a | The guard really forbids use-after-shutdown | Mutation — an `NVML` call after `drop(session)` — rejected with `E0505`; `NVML` traces identical to the previous build; live tests on Windows and Linux |
| 9b | The table renderer is byte-identical | Probes on fixtures including multibyte names and empty inputs: 1,247 bytes, identical |

Ubuntu WSL2 ran the same gates after items 5, 7 and 9a (the ones compiling code Linux builds) and
once more at the end; MSRV 1.88 clippy ran after items 2, 8 and 9a, and at the end. Not verifiable
here: macOS (`metal.rs` untouched by all nine items; the macOS CI leg covers it on push).

### Consistency pass — what the per-commit gates could not see

After the ten commits, everything part 1 touched was re-checked mechanically, on the principle that
a green gate proves only what it was built to watch:

- **Stale lint suppressions.** Every `#[allow]` in the touched files was temporarily turned into
  `#[expect]`, which makes the compiler report each one that suppresses nothing, then compiled in
  eight configurations (Windows / Linux × stable / MSRV 1.88 × default / all features). Found: one
  `unsafe_code` allow item 5 left behind, and eleven `missing_panics_doc` allows in the `hmn`
  modules that never did anything. Two others fire only on MSRV 1.88 and were rightly kept — a
  stable-only scan would have removed them.
- **`SAFETY` coverage.** Clippy's `undocumented_unsafe_blocks` (not in the crate's lint set) found
  ten blocks in `nvml.rs` whose comment was shared or separated from the `unsafe`; `dxgi.rs` and
  `pdh.rs` were already clean. Now zero, on both platforms.
- **Private-item docs.** Rendering with `--document-private-items` on both platforms found six
  platform-gated intra-doc links in `nvml.rs` that break on the other platform — invisible to the
  normal doc gate, which renders public items only.
- **Stale references.** Every backticked identifier in the touched files' comments was checked for
  a remaining occurrence in code: none refer to anything removed.
- **Annotations, packaging, figures.** Every line added since the audit was scanned for an
  unannotated `as` cast or borrow conversion (none); `cargo package --list` confirms the published
  crate carries all of `src/bin/hmn/`; and every figure quoted in the CHANGELOG was re-derived
  from history, correcting two (item 7's line split, item 8's starting size).

The pass also surfaced the uniform test-module lint preamble (`unwrap_used`, `expect_used`,
`missing_docs_in_private_items`) as carrying dead entries — crate-wide boilerplate predating part
1. At the maintainer's call it was then trimmed crate-wide: the same `#[expect]` check, widened to
twelve configurations (macOS type-checked via `aarch64-apple-darwin`), removed 33 of 44 entries
and five preambles outright; `CONVENTIONS.md` now states the rule.

### Withdrawn on inspection

The audit suggested rewording `ROADMAP.md`'s speculative *"Unified `spill_condition` core"* entry,
on the grounds that it *"reads as though a duplicate exists"*. Re-read in full while doing item 4,
the entry is accurate: it already says *"only the dedicated-threshold arithmetic is actually
shared"*, and its real motivation is letting `snapshot_is_spilling` honour the threshold overrides
`SpillTracker` exposes — a behaviour question, not a duplication one. Left unchanged.

### Follow-ups observed

**`pdh.rs` error wording — ✅ resolved after part 1, at the maintainer's call.** Every
`HypomnesisError::Pdh` message used the form `"<Api> failed: 0x…"`. Item 5 moved one without
rewording it, since changing one would break the file's internal consistency. The note above first
framed the target as `CONVENTIONS.md`'s `"failed to <verb>: {e}"`; on a closer read that form is
for wrapping a Rust error value, while a failed FFI *status code* takes the validation form
`<noun> <problem> (<context>)` — the form `ram.rs` already uses (`K32GetProcessMemoryInfo failed
(GetLastError = …)`, `task_info(…) failed (kern_return = …)`). All eight messages now match it,
e.g. `PdhOpenQueryW failed (PDH_STATUS = 0x…)`. No code, test or current doc matched on the old
text.

The text tables measure column widths in **bytes** (`column_width` uses `str::len`) but `{:<w$}`
pads in **chars**, so a row with a non-ASCII process name gets more padding than it needs and its
columns can drift right. Item 9 preserved this byte-for-byte on purpose — changing it changes
`hmn ps` and `hmn watch` output — so it is a separate, visible decision. A fix would measure
widths in chars, or in display columns if East Asian wide characters are to line up too.

---

## Part 2 — `hmn watch` selects by identity

### Origin — `--follow-new` fixed *when* `watch` selects, not *what*

[`docs/dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md`](dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md)
(candle-mi, 2026-09-21, corrected 2026-09-26): four `hmn watch --follow-new --top 3` captures,
committed to a public repository as experimental record, were 73.9% desktop rows — selection is by
VRAM rank, which cannot say "follow this program". Requests, in the report's own priority order:
`--filter` composing with `--follow-new`; `--min <SIZE>` on `watch`; the active criterion on the
`.err` header; and, as an observation, a `{"kind":"start", ...}` record making truncated captures
detectable from the file alone.

### Design decisions taken before starting

- **`--filter` is a case-insensitive substring, repeatable (OR).** Windows process names are
  case-insensitive, and the report's own example is a stem without `.exe`; a case-sensitive match
  would silently miss `Python.exe` — the very silent-drop shape the report warns about.
- **Unresolved names: sticky last-resolved name, plus a one-shot announcement.** A followed PID
  whose name flickers to `[protected]` keeps matching on the last name it resolved to; a PID that
  never resolved is announced once on stderr rather than dropped silently.
- **The `start` record is always the first line of `--json`.** Its value is being present on the
  run nobody expected to be cut short; an opt-in flag is forgotten exactly then. It is v0.2.12's
  one deliberate wire-format addition — additive under Principle 2, and no known consumer reads the
  stream positionally.
- **`hmn watch` only.** A `--filter` on `hmn ps` would be the fourth `ps` filter, which is what
  `ROADMAP.md` gates its speculative `PsFilters` refactor on; that deserves its own slot.
- **One `Selection` value** drives the selection, the header and the `start` record, so what a
  capture says it selected and what it selected cannot drift apart.

### Scope

| # | Item | Kind | Status |
|---|---|---|---|
| 1 | `Selection` type; `footprint_bytes` shared by `ps --min`, `ps --sort total` and `watch --min` | refactor | ✅ |
| 2 | `hmn watch --filter <PATTERN>` + criterion on the header (requests 1 and 3) | feature | ✅ |
| 3 | `hmn watch --min <SIZE>` + criterion on the header (requests 2 and 3) | feature | ✅ |
| 4 | `{"kind":"start", ...}` first record of `--json` (observation 1) | feature | ✅ |
| 5 | README, FAQ, tutorial, roadmap close-out | docs | ✅ |

The report's second smaller observation — `--top 3 --follow-new` churning through 42 handovers as
the workload restarted — is not an item of its own. The report names the remedy itself: a filter
holds the followed set at the workload, which `--filter` now does. Its claim that a filter would
also save queries it had already withdrawn, correctly: the listing is per-interval regardless.

### Deviations from the plan, and why

- **One more refactor commit: `json_string_or_null` / `json_value_or_null`.** The `start` record
  was about to add four more copies of a four-line "optional JSON value" idiom that already
  existed ten times — short enough to slip under part 1's six-line detector window. Extracted
  first, byte-identical.
- **`argv[0]` is reduced to its file name.** The first live run of the `start` record recorded the
  full executable path — the operator's user name and directory layout — in a record designed to
  be committed publicly, the very privacy concern the report raises. Caught live, fixed before
  commit.
- **A dead lint allow on the live tests' `spillforge_path` helper**, in all three copies (one of
  them new in item 4), found by the same `#[expect]` check part 1 introduced. Removed in its own
  commit.

### Verification

- Every commit passed the full gate set on Windows; MSRV 1.88 clippy ran after items 2 and 3.
- Unit tests: 29 new across part 2 (295 → 324; 289 → 324 over the release) — the guards, the header clauses
  (with the no-flag strings pinned byte-identical to v0.2.11's), filter-then-top-N ordering, the
  OR of patterns, the sticky name through a `[protected]` flicker, unmatchable reporting and its
  once-only notice, `--min`'s `used + shared` semantics and its place before the filter, the exact
  bytes of the `start` record in each mode, and `argv[0]` reduction on both platforms.
- Live on the RTX 5060 Ti: `--filter FIREFOX` follows only `firefox.exe`; no match exits `2`
  one-shot and waits under `--follow-new`; `--filter dwm --min 1GiB` composes; each explicit-PID
  guard exits `2`; the text header and the `--json` first line carry the criterion.
- A new `#[ignore]` live test, `tests/live_watch_filter.rs`, is the report's own regression case:
  `--follow-new --top 3 --filter SpillForge` over a real `spillforge` run records only
  `spillforge.exe` — 100% workload rows — with the `start` record first. The existing
  `live_watch` and `live_watch_follow_new` tests still pass with the new first line in the
  stream.

### Consistency pass

After the five feature commits, part 2's code was re-checked the way part 1's was. Mechanically:
the `#[expect]` scan in all twelve configurations (only the two known MSRV-only allows fire
conditionally), `--document-private-items` on both platforms, stale backticked identifiers, and the
unannotated-conversion scan — all clean. The duplicate detector's three new groups were all the
live tests' `spillforge_path` helper, now in three copies. By reading, eleven findings:

- **Docs that drifted while the design grew.** `Selection`'s own doc named two consumers; it has
  four (the header, the nothing-to-select messages and the `start` record's `selection` object as
  well). `describe` promised byte-identity "without `--filter`" where it holds without `--min`
  either; `write_json` claimed to carry "the same fields" `describe` words, which it does not
  (`pids`, `top` in explicit mode). Two doc blocks kept a rewrap artifact.
- **The sticky name's limit, now stated.** `matchable_name` documents that a reused PID whose new
  owner has no resolvable name inherits the old one — best-effort, like `hmn watch`'s PID-reuse
  handling generally.
- **`#[must_use]` with a reason** on `unmatchable_notices`, whose call also marks the PIDs
  announced: dropping the result loses those notices for good.
- **`parse_filter_pattern` moved to `format.rs`**, beside `parse_duration` and `parse_size_bytes`
  — the other clap value parsers.
- **`json_string`**, for the `start` record's always-present strings, which were spelled
  `json_string_or_null(Some(..))`; `json_string_or_null` is now built on it.
- **A stale test comment** in `main.rs` still said `run_watch` rejects explicit PIDs with
  `--follow-new`; `Selection::new` does, and is unit-tested for it.
- **`spillforge_path` shared.** Three identical copies of the live tests' fixture helper is past
  what the duplicate-code audit judged "defensible at this size" for two; it now lives in
  `tests/common/mod.rs`, pulled in with `mod common;` (not a test binary of its own), and the
  comment justifying the duplication is gone with it. In its own commit.

No behaviour changed: every exact-output test passes unchanged, and `hmn watch --help` is
byte-identical.

### Found after the consistency pass

- **Linux names were cut to 15 bytes, so `--filter` could not match a long program name.** The
  Linux listing named `NVML`'s rows from `/proc/<pid>/comm`, which the kernel truncates: the
  report's own example, `--filter figure13_newline_patch`, would have matched nothing. No gate
  could see it — the unit tests build rows by hand, and under WSL2 `NVML` returns no per-process
  rows at all, so the live checks ran on Windows only. Fixed in the library (a new
  `src/gpu/proc_name.rs`): a `comm` at the limit is extended from the `exe` link's file name, else
  `argv[0]`'s, when that starts with it. Two tests read the real `/proc` of a long-named process
  (the test binary itself, and a child started through a symlink) and fail with the fix disabled.

### At release

- Bump `Cargo.toml` to `0.2.12`; flip this roadmap's status and the dogfooding report's `Status`
  line (`✅ Resolved in v0.2.12`), per the dogfooding style guide.
- Rotate the README's "what's new" banner (new 🆕, previous to 🚀, drop the oldest of three).
- Refresh the `start`-record sample in the watch tutorial's Step 5, captured from a pre-release
  build and so reading `"hmn_version":"0.2.11"`.

---

## References

- [`docs/audits/2026-09-26-duplicate-code-audit.md`](audits/2026-09-26-duplicate-code-audit.md) —
  the audit part 1 remediates, with per-item line references and the detector's method.
- [`docs/audits/2026-08-17-codebase-documentation-audit.md`](audits/2026-08-17-codebase-documentation-audit.md)
  — the previous audit; three of its fifteen items are closed by part 1's item 4, and its item 3.1
  is completed by items 3 and 8.
- [`docs/dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md`](dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md)
  — part 2's origin.
