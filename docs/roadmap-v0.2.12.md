# `hypomnesis` v0.2.12 — roadmap

> *Clean the base first, then teach `hmn watch` to follow a process by name.*

**Status: in progress.** Part 1 (audit remediation) ✅ done 2026-09-26 — nine items in ten commits,
then a `PDH` error-wording follow-up and a consistency pass; none pushed yet. Part 2 (dogfooding
features) not started.

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

Recorded, not changed: the uniform test-module lint preamble (`unwrap_used`, `expect_used`,
`missing_docs_in_private_items`) includes entries that suppress nothing in some modules —
`missing_docs_in_private_items` in all of them. It is crate-wide house boilerplate, predating part
1, and trimming it is a style decision for the whole crate.

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

*Not started.* Origin:
[`docs/dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md`](dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md)
(candle-mi, 2026-09-21, corrected 2026-09-26). Requests, in the report's own priority order:
`--filter <SUBSTRING>` composing with `--follow-new`; `--min <SIZE>` on `watch`; the active criterion
echoed on the `.err` header; and, as an observation, a `{"kind":"start", ...}` record making
truncated captures detectable from the file alone.

---

## References

- [`docs/audits/2026-09-26-duplicate-code-audit.md`](audits/2026-09-26-duplicate-code-audit.md) —
  the audit part 1 remediates, with per-item line references and the detector's method.
- [`docs/audits/2026-08-17-codebase-documentation-audit.md`](audits/2026-08-17-codebase-documentation-audit.md)
  — the previous audit; three of its fifteen items are closed by part 1's item 4, and its item 3.1
  is completed by items 3 and 8.
- [`docs/dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md`](dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md)
  — part 2's origin.
