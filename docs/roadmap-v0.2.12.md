# `hypomnesis` v0.2.12 — roadmap

> *Clean the base first, then teach `hmn watch` to follow a process by name.*

**Status: in progress.** Part 1 (audit remediation) under way; part 2 (dogfooding features) not
started.

---

## Why v0.2.12 (and not v0.3.0)

Both parts are patch-safe under the crate's own rule (`ROADMAP.md`, Principle 2: *"New variants and
fields land in patch releases. Type-shape changes … are minor bumps, never patches."*).

- **Part 1** is internal: eight behaviour-preserving refactors and one robustness fix. No public item
  is added, removed or re-signatured; the `hmn` output contract — text and `--json` — stays
  byte-identical, guarded by the existing exact-string tests plus one new key-parity test.
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

### Withdrawn on inspection

The audit suggested rewording `ROADMAP.md`'s speculative *"Unified `spill_condition` core"* entry,
on the grounds that it *"reads as though a duplicate exists"*. Re-read in full while doing item 4,
the entry is accurate: it already says *"only the dedicated-threshold arithmetic is actually
shared"*, and its real motivation is letting `snapshot_is_spilling` honour the threshold overrides
`SpillTracker` exposes — a behaviour question, not a duplication one. Left unchanged.

### Follow-ups observed, not in scope

Every `HypomnesisError::Pdh` message in `src/gpu/pdh.rs` uses the house form `"<Api> failed:
0x…"` rather than `CONVENTIONS.md`'s `"failed to <verb>: {e}"`. Item 5 moves one of them without
rewording it, since changing one would break the file's internal consistency. A file-wide pass is a
separate decision: the strings are user-visible through `HypomnesisError`'s `Display`.

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
