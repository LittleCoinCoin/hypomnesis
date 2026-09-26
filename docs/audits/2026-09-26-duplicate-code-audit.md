# hypomnesis — Duplicate-Code Audit

**Date:** 2026-09-26
**Audited at:** commit `585fffd` (v0.2.11, tagged and published); working tree carries one modified
file, `docs/dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md`, and no source changes
**Auditor:** Claude Code (mechanical near-duplicate detection over every tracked Rust file, then a
targeted read of every cluster it flagged, plus verification of the 2026-08-17 audit's items)
**Requested focus:** duplicate code, ahead of implementing
[`dogfooding-watch-filter-by-identity.md`](../dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md)

---

## Scope and method

- **Mechanically scanned:** all 13 `src/` files (9,070 production lines + 3,617 lines of inline
  `#[cfg(test)]` modules = 12,687) and all 6 `tests/` files (1,176 lines), with a sliding-window
  hash detector: comments stripped, whitespace collapsed, brace-only and attribute-only lines
  dropped, then every window of *N* normalised lines hashed and collided. Run at N = 20, 14, 8 and
  6, once including and once excluding inline test modules. Each surviving cluster was then read in
  full in its source context — no finding below rests on the hash alone.
- **Read in full while chasing clusters:** `src/gpu/dxgi.rs`, `src/gpu/nvidia_smi.rs`,
  `src/spill.rs`'s condition core, `src/report.rs`, `GpuDeviceInfo`'s `report`-feature formatters,
  `src/bin/hmn.rs`'s formatter and watch-state sections, both `tests/live_watch*.rs`, and the
  `nvml.rs` / `pdh.rs` entry points.
- **Verified live (Windows 11, this machine):** `cargo fmt --check` ✅ · `cargo clippy
  --all-targets --all-features -- -D warnings` ✅ · `cargo test --all-features` ✅ (**289 passed,
  0 failed, 18 `#[ignore]`-gated live tests not run** — up from 239 at the last audit) ·
  `cargo doc --all-features --no-deps` under `RUSTDOCFLAGS="-D warnings"` ✅ ·
  `cargo check --no-default-features` ✅.
- **Not verified here:** the Ubuntu/WSL2 leg, macOS builds, and the `#[ignore]`-gated live-GPU
  tests.

**The headline mechanical result, stated before the findings because it frames all of them: there
is no copy-paste in this codebase.** At a 20-line window: zero duplicate groups. At 14 lines: zero.
Every cluster below is 6–13 normalised lines of *structural* repetition — an FFI preamble, a wire-
format field list, a table-rendering shape — not a duplicated algorithm. Nothing was found that a
reasonable reviewer would call cut-and-paste, and no duplicated block was found to have drifted
into disagreement.

**Overall assessment.** The five gates pass clean and the crate's own anti-duplication instincts are
visible and working: `column_width`, `json_escape`, `write_episodes_json`,
`default_dedicated_threshold`, `select_top_n_pids` and `ps_row_comparator` all exist precisely so
two call sites cannot drift, several with rustdoc saying so. The duplication that remains is
concentrated where a shared helper would have to cross an `unsafe` FFI boundary or a `cfg` gate —
the places where extracting one is genuinely harder. One cluster has already cost the project a
defect (§P1.1: a v0.2.10 fix applied to 2 of 6 sites), one encodes a consumer-facing wire contract
in four places with no test that they agree (§P2.1), and the rest are maintenance load rather than
risk. Of the fifteen items from the 2026-08-17 audit, eleven are fully closed, two are partial and
two are still open.

---

## Status of the 2026-08-17 audit's items

Checked every item; 11 fully closed, 4 open or partial.

| Item | Status |
|---|---|
| 1.1 `hmn --help` pre-v0.2.8 `?` semantics | ✅ closed — `long_about` now uses `[exited]`/`[protected]` ([hmn.rs:118-133](../../src/bin/hmn.rs#L118-L133)) |
| 1.2 `hmn.rs` module docs say `cli` is default-off | ✅ closed ([hmn.rs:3-4](../../src/bin/hmn.rs#L3-L4), [:40](../../src/bin/hmn.rs#L40)) |
| 1.3 `gpu_processes()` rustdoc Limitations | ✅ closed — rewritten to the two-stage-fallback contract ([mod.rs:344-357](../../src/gpu/mod.rs#L344-L357)) |
| 2.1 No macOS leg in CI | ✅ closed — `os: [ubuntu-latest, windows-latest, macos-latest]` ([ci.yml:22](../../.github/workflows/ci.yml#L22)) |
| 2.2 ROADMAP "not yet published" contradictions | ✅ closed |
| 2.3 `CONVENTIONS.md` stale default-feature list | ✅ closed ([CONVENTIONS.md:402](../../CONVENTIONS.md#L402)), and the `cli` row now lists `ctrlc` ([:385](../../CONVENTIONS.md#L385)) |
| 2.4 CI never builds non-default feature combos | ✅ closed — both `--no-default-features` checks present ([ci.yml:41-44](../../.github/workflows/ci.yml#L41-L44)) |
| 2.5 `publish.yml` no tag/version guard | ✅ closed ([publish.yml:33-38](../../.github/workflows/publish.yml#L33-L38)) |
| 3.1 DXGI walk aborts on one bad adapter | ⚠️ **partial — 2 of 6 sites fixed.** See §P1.1 |
| 3.2 `HypomnesisError::Io` unconstructible | ✅ closed by documentation ([error.rs:86-90](../../src/error.rs#L86-L90)) |
| 3.3 NVML 64-process cap truncates silently | ✅ closed — retries with a larger heap buffer ([nvml.rs:80-87](../../src/gpu/nvml.rs#L80-L87)) |
| 3.4 Stale "planned for v0.2" forward references | ⚠️ **partial** — `nvml.rs` fixed; [`snapshot.rs:233`](../../src/snapshot.rs#L233) still reads *"A long-lived `NVML` context is planned for v0.2."* and is rustdoc-visible on docs.rs |
| 3.5 `Cargo.toml` mentions one builder of three | ❌ **open** — [`Cargo.toml:81`](../../Cargo.toml#L81) still says *"exposes a `GpuDeviceInfoBuilder`"*; `SpillReportBuilder` and `GpuProcessEntryBuilder` are also exposed |
| 3.6 `CHANGELOG.md` bracketed versions have no link definitions | ❌ **open** — still no link-reference block |
| 3.7 `hmn` can print nothing on a half-broken stack | ✅ closed — `format_none_readable_message` ([hmn.rs:425-427](../../src/bin/hmn.rs#L425-L427)) |

---

## P1 — Duplication that has already caused a defect

### 1.1 The v0.2.10 DXGI abort-on-bad-adapter fix reached 2 of 6 identical walks

`src/gpu/dxgi.rs` contains **six** independent `CreateDXGIFactory1` + `EnumAdapters1` walks. The
2026-08-17 audit's item 3.1 named two of them, and v0.2.10 fixed exactly those two. The other four
still carry the pre-fix shape.

| walk | lines | on a failed `cast::<IDXGIAdapter>()` / `GetDesc()` |
|---|---|---|
| [`query`](../../src/gpu/dxgi.rs#L75) | 75-144 (70) | `.ok()?` — **aborts the whole walk** ([:89](../../src/gpu/dxgi.rs#L89), [:93](../../src/gpu/dxgi.rs#L93)) |
| [`adapter_name`](../../src/gpu/dxgi.rs#L160) | 160-192 (33) | `.ok()?` — **aborts** ([:170](../../src/gpu/dxgi.rs#L170), [:173](../../src/gpu/dxgi.rs#L173)) |
| [`adapter_luid`](../../src/gpu/dxgi.rs#L213) | 213-239 (27) | `.ok()?` — **aborts** ([:225](../../src/gpu/dxgi.rs#L225), [:228](../../src/gpu/dxgi.rs#L228)) |
| [`adapter_dedicated_video_memory`](../../src/gpu/dxgi.rs#L263) | 263-292 (30) | `.ok()?` — **aborts** ([:275](../../src/gpu/dxgi.rs#L275), [:278](../../src/gpu/dxgi.rs#L278)) |
| [`enumerate_non_nvidia`](../../src/gpu/dxgi.rs#L333) | 333-434 (102) | ✅ `let Ok(..) else { raw_idx += 1; continue }` (v0.2.10) |
| [`device_count`](../../src/gpu/dxgi.rs#L442) | 442-488 (47) | ✅ skip-and-continue (v0.2.10) |

The four unfixed walks are the NVIDIA-filtered ones, and their rustdoc says so out loud —
`adapter_luid` and `adapter_dedicated_video_memory` are both documented as *"Walks `EnumAdapters1`
with the same NVIDIA-filter rule as [`query`]"*. Three near-copies of a walk whose fourth copy was
patched is the textbook cost of structural duplication.

**Impact.** Different from the fixed pair, and in one respect worse. `enumerate_non_nvidia` and
`device_count` *undercounted* on a mid-walk failure; these four *return `None` outright*, so on a
machine where an adapter earlier in the raw enumeration order fails `GetDesc` — an iGPU with a
half-installed driver is the realistic case — `hmn` gets no DXGI reading at all for a healthy
NVIDIA GPU sitting behind it in the walk. It degrades to the NVML/`nvidia-smi` path rather than
reporting a wrong number, so this is a robustness gap and not a correctness bug, and on healthy
hardware these calls essentially never fail. But it is the *same* gap the project already decided
was worth closing.

**Fix.** Two options, and the second is the reason this is filed under duplication rather than
robustness:

1. *Narrow* — apply the same `let Ok(..) else { raw_idx += 1; continue; }` shape plus a
   `debug-output` trace to the four walks. ~20 lines, mechanical, matches the existing fixed pair
   exactly.
2. *Structural* — extract one internal iterator, e.g. `fn nvidia_adapters() -> impl Iterator<Item =
   (u32, IDXGIAdapter, DXGI_ADAPTER_DESC)>`, and rewrite all six walks over it. The
   skip-on-failure policy, the `NVIDIA_VENDOR_ID && DedicatedVideoMemory > 0` filter, the
   `raw_idx`/`nvidia_count` double index and the UTF-16 name trim
   ([:104-109](../../src/gpu/dxgi.rs#L104-L109), duplicated at
   [:386-391](../../src/gpu/dxgi.rs#L386-L391)) then exist once. This collapses ~309 lines of walk
   into an iterator plus six short bodies, and makes a seventh walk safe to add.

Do 1 now if v0.2.12 is meant to stay small; 1 and 2 are not mutually exclusive, and 2 wants its own
release slot because it touches every `unsafe` block in the file.

---

## P2 — Duplication that states one contract in several places

These are the clusters where the duplicated text *is* a contract a consumer depends on, so
divergence would be a wire-format or output-format bug rather than a tidiness problem.

### 2.1 The `SpillReport` JSON field block is spelled four times, and no test asserts they agree

The nine-field adapter-level JSON object that `hmn spill --json` and `hmn watch --json` both emit is
written out, in full, in four independent places:

| # | site | form |
|---|---|---|
| 1 | [`SPILL_JSON_UNMEASURABLE`](../../src/bin/hmn.rs#L972) (hmn.rs:972-978) | a `concat!` of four string literals |
| 2 | [`format_spill_json`](../../src/bin/hmn.rs#L1244) (hmn.rs:1244) | one `write!` format string |
| 3 | [`format_watch_summary_json`](../../src/bin/hmn.rs#L1928), `Some` arm (hmn.rs:1928) | one `write!` format string |
| 4 | `format_watch_summary_json`, `None` arm ([hmn.rs:1942](../../src/bin/hmn.rs#L1942)) | one all-zeros string literal |

I extracted the key sequence from each mechanically. **All four agree today** — `measurable`,
`spilled`, `observations`, `baseline_shared_bytes`, `peak_shared_bytes`, `peak_dedicated_bytes`,
`dedicated_limit_bytes`, `total_spill_duration_ms`, `episodes`, in that order. Nothing has drifted.

What is missing is the guard. The relevant tests assert only the ends of the string:

- [`hmn.rs:3365-3366`](../../src/bin/hmn.rs#L3365-L3366) — `SPILL_JSON_UNMEASURABLE.starts_with("{\"measurable\":false,\"spilled\":false,")` and `.ends_with("\"episodes\":[]}\n")`
- [`hmn.rs:4388`](../../src/bin/hmn.rs#L4388) — `starts_with(r#"{"kind":"summary","measurable":false,"spilled":false,"observations":0,"#)`
- [`hmn.rs:4399`](../../src/bin/hmn.rs#L4399) — `contains(r#""per_pid":[]"#)`

So the first two or three fields and the last are pinned; **the middle five are asserted nowhere**.
Insert, rename or reorder a field in two of the four spellings and every test still passes, while
`hmn spill --json` and `hmn watch --json` — or the measurable and unmeasurable paths of the *same*
command — silently stop producing the same shape. The `None`-arm's own comment concedes the
duplication and reconciles it by prose (*"emits the same all-zeros shape `SPILL_JSON_UNMEASURABLE`
uses, so scripted consumers always parse one shape either way"*) rather than by code.

This is the crate's externally-visible contract: candle-mi commits `hmn watch --json` output to a
public repository as experimental record, and the report this audit precedes parses those files
field by field.

**Fix, cheapest first.** A test comparing the four key sequences — the exact check I ran, ~15 lines
of `Vec<&str>` comparison over the emitted strings — closes the risk without touching production
code and should land regardless. Beyond that, one `fn write_spill_report_fields(out: &mut String,
report: Option<&SpillReport>)` would replace all four, following the precedent
`write_episodes_json` ([hmn.rs:1214](../../src/bin/hmn.rs#L1214)) already set for the nested array —
and note that helper *does* have a parity test
([`write_episodes_json_matches_format_spill_json_episodes`](../../src/bin/hmn.rs#L3387)), which is
the pattern to copy.

### 2.2 The SPILL-column honesty mapping is written twice, verbatim, comment and all

[`hmn.rs:891-898`](../../src/bin/hmn.rs#L891-L898) (`format_ps_table`) and
[`hmn.rs:1739-1746`](../../src/bin/hmn.rs#L1739-L1746) (`format_watch_rows_text`):

```rust
// "?" (not "no") for `None` — the same "can't tell" convention used
// elsewhere for unresolved process names, so an operator never
// mistakes "not measurable here" for "measured, not spilling".
let spill_cells: Vec<&str> = rows
    .iter()
    .map(|r| match r.spilling {
        Some(true) => "SPILL",
        Some(false) => "no",
        None => "?",
    })
    .collect();
```

The three-arm mapping *is* the v0.2.11 SPILL honesty contract — the headline of that release, and
the thing `ROADMAP.md` describes as never collapsing "can't tell" into "not spilling". It is encoded
in two places, and the justifying comment is duplicated alongside it (the second copy reworded to
point at the first). No test asserts the two surfaces render the same glyphs. A `const fn
spill_cell(s: Option<bool>) -> &'static str` is four lines and removes the possibility entirely; it
also gives the contract one place to be tested and one place to be documented.

---

## P3 — Mechanical duplication: maintenance load, not risk

Ordered by ratio of duplicated lines to lines that actually differ.

### 3.1 `nvidia-smi` spawn-and-diagnose: 24 lines duplicated, 2 lines different

[`query`](../../src/gpu/nvidia_smi.rs#L47) (47-100) and
[`query_compute_apps`](../../src/gpu/nvidia_smi.rs#L205) (205-256) open with the same
`Command::new("nvidia-smi")` + `--format=csv,noheader,nounits` + `--id={idx}` + `.output()`
sequence, then the same four-arm `match` over `(Ok/Err, debug-output on/off)` with the same
`String::from_utf8_lossy(&o.stderr)` diagnostic. The only differences are the `--query-*` argument
and the label inside the `eprintln!`. This is the densest duplicate in the crate, and it is also the
one where the duplication is most awkward, because the four-arm `cfg`-gated match is written twice:
a `cfg` mistake would have to be found twice.

**Fix:** `fn run_smi(query: &str, idx: u32, what: &str) -> Option<String>` returning the
lossy-decoded stdout. Both call sites become three lines. ~35 lines removed, no behaviour change,
and the `cfg` arms exist once.

### 3.2 `PdhOpenQueryW` into an RAII guard: written twice

[`pdh.rs:503-516`](../../src/gpu/pdh.rs#L503-L516) (`collect_segmented_rows`) and
[`pdh.rs:785-796`](../../src/gpu/pdh.rs#L785-L796) (the `AdapterMemQuery` open path) repeat the same
zero-init handle → `PdhOpenQueryW(PCWSTR::null(), 0, ..)` → status check → wrap in `QueryGuard`
sequence, including the `format!("PdhOpenQueryW failed: 0x{status:08X}")` error text. The second
site's `// SAFETY:` comment already says *"same documented form as collect_segmented_rows"* — the
duplication is acknowledged in the source.

**Fix:** `fn open_query() -> Result<QueryGuard>`. The `unsafe` block and its SAFETY justification
then exist once, which is exactly what `CONVENTIONS.md`'s `unsafe`-scoping policy is trying to buy.
Low risk: `QueryGuard` already exists and already owns the handle.

### 3.3 Three NVML entry points each hand-roll the load / init / shutdown pairing

[`query`](../../src/gpu/nvml.rs#L262) (262-366, 105 lines),
[`list_compute_processes`](../../src/gpu/nvml.rs#L668) (668-850, 183) and
[`device_count`](../../src/gpu/nvml.rs#L858) (858-893, 36) each repeat:
`Library::new(NVML_LIB_PATH)`, `lib.get(b"nvmlInit_v2\0")`, `lib.get(b"nvmlShutdown\0")`, call init,
check the return, and then call `shutdown()` on **every** subsequent return path. The module doc
([nvml.rs:15](../../src/gpu/nvml.rs#L15)) documents the per-entry-point pairing as deliberate, and
two of the three carry an identical hand-written comment — *"From here, every return path MUST call
shutdown to balance the init."*

**I verified the pairing is correct.** I traced every `return` and every `?` after the init call in
all three functions: no `?` escapes the init, and each early return calls `shutdown()` first. This
confirms the 2026-08-17 audit's finding and it is still true at v0.2.11.

The finding is that the invariant is upheld by a comment repeated three times instead of by the type
system. A `struct NvmlSession { lib, shutdown }` with a `Drop` impl would make the pairing
structural, delete three copies of the warning comment, and let the three entry points use `?`
freely — which would also shorten `list_compute_processes` materially. This is the highest-value
refactor in the file and the one with the most `unsafe` in its blast radius; it wants its own release
slot and its own adversarial review pass, not a ride-along.

### 3.4 Three hand-rolled column-aligned table renderers

[`format_ps_table`](../../src/bin/hmn.rs#L862) (65 lines, 6 columns),
[`format_watch_rows_text`](../../src/bin/hmn.rs#L1717) (57, 7) and
[`format_watch_per_pid_block`](../../src/bin/hmn.rs#L1826) (75, 6) share one shape: build one `Vec`
of cells per column, call `column_width` once per column, then `zip` the vectors six or seven deep
and `writeln!`. 197 lines total, of which the genuinely distinct content — headers, which field
feeds which column, the row prefix — is perhaps 40.

The shared `column_width` helper ([hmn.rs:2429](../../src/bin/hmn.rs#L2429)) already exists and is
used 22 times, so the *arithmetic* is deduplicated; what repeats is the plumbing, and the
`.zip(..).zip(..).zip(..).zip(..).zip(..).zip(..)` chains are the least pleasant code in the crate
to read or extend.

**Honest assessment:** a small internal `Table { headers: Vec<&str>, rows: Vec<Vec<String>> }` with
a `render(prefix) -> String` would collapse all three and make a fourth table trivial. But this is
~200 lines of pure-function refactor in the file that holds the CLI's entire output contract, all
three renderers are well covered by exact-string unit tests, and the payoff is legibility rather
than risk reduction. Worth doing when a fourth table is next needed — which, per §"Bearing on
v0.2.12" below, may be sooner than it looks. Not worth doing on its own.

### 3.5 `GpuDeviceInfo::format_free` / `format_total` / `format_used`

[snapshot.rs:410-428](../../src/snapshot.rs#L410-L428),
[:448-464](../../src/snapshot.rs#L448-L464), [:476-489](../../src/snapshot.rs#L476-L489) — 50 lines
across three public `report`-feature methods that differ only in the label word and which field(s)
they divide. Each repeats the same `#[allow(clippy::cast_precision_loss, clippy::as_conversions)]` +
`/ 1_048_576.0` conversion and the same `name.as_deref().map_or(String::new(), |n| format!("
[{n}]"))` suffix block.

These are public API with stable output asserted by six unit tests, so the safe deduplication is
internal only: one private `fn name_suffix(&self) -> String` and one `fn mib(bytes: u64) -> f64`,
leaving the three public signatures and their exact output untouched. Small, safe, and it removes
three copies of a `clippy::as_conversions` allow.

---

## Deliberate duplication — verified, leave alone

Recorded so a future pass doesn't re-derive it or "fix" it.

- **`tests/live_watch.rs` and `tests/live_watch_follow_new.rs` duplicate `spillforge_path()` and the
  summary-line extraction** ([live_watch.rs:34-55](../../tests/live_watch.rs#L34-L55),
  [live_watch_follow_new.rs:32-57](../../tests/live_watch_follow_new.rs#L32-L57)). The second copy
  carries a rustdoc explaining the choice, which is the right instinct — two `#[ignore]`-gated
  integration binaries, twenty lines of fixture path, sharing would not pay.
  **One factual correction to that comment:** it claims *"Cargo has no lightweight way to share a
  helper between them without a `[lib]`/`dev-dependencies` shim"*. Cargo does have one —
  `tests/common/mod.rs` plus `mod common;` in each file, which is the standard idiom and, because it
  lives in a subdirectory, does not become a third test binary. The *decision* to duplicate is still
  defensible at this size; the stated *reason* is not, and the comment will mislead the next person
  who wants to share something bigger. Reword rather than refactor.
- **`src/gpu/metal.rs:386-395` and `:473-481`** — the `(0..LEDGER_TEMPLATE_BUF_CAP).map(|_| unsafe {
  core::mem::zeroed::<T>() }).collect()` + `count: i32` cast idiom, over two *different*
  `#[repr(C)]` types (`LedgerTemplateInfo`, `LedgerEntryInfo`). Deduplicating needs a generic or a
  macro and would obscure two distinct SAFETY arguments. Correctly left alone.
- **The `#[cfg(test)] #[allow(clippy::unwrap_used, clippy::expect_used,
  clippy::missing_docs_in_private_items)]` preamble, 4 occurrences** (`nvidia_smi.rs:326`,
  `pdh.rs:1197`, `report.rs:150`, `snapshot.rs:769`, plus the `cfg(all(test, target_os = "linux"))`
  variant in `nvml.rs:900` and `ram.rs:394`). Idiomatic per-module test-lint relaxation; there is no
  mechanism to share it and no reason to want one.
- **The spill condition is *not* duplicated, despite appearances.**
  [`default_dedicated_threshold`](../../src/spill.rs#L232) is shared by `fold`
  ([spill.rs:495-497](../../src/spill.rs#L495-L497)) and `saturated_with_shared_floor`
  ([spill.rs:258-266](../../src/spill.rs#L258-L266)), with rustdoc on the helper stating the reason —
  *"so the two can't silently drift apart on this one piece of arithmetic"*. The only divergence
  between the two conditions is deliberate and semantic: the tracker uses growth-over-baseline, the
  single-snapshot path uses an absolute floor, because a one-shot listing has no baseline. Worth
  saying plainly because `ROADMAP.md`'s speculative v0.3.0 entry *"a unified `spill_condition`
  core"* reads as though a duplicate exists; what remains to unify is a deliberate semantic
  difference, not repeated code. Consider rewording that ROADMAP entry.

---

## Non-duplication findings

### 4.1 `MemoryReport`'s public API labels MiB values "MB" and, alone among the crate's surfaces, doesn't say so

The crate is otherwise scrupulous about this. [`Snapshot::ram_mb`](../../src/snapshot.rs#L351)
documents *"as megabytes (`bytes / 1_048_576`)"*;
[`GpuDeviceInfo::format_free`](../../src/snapshot.rs#L404) says *"`MB` here means `MiB` (`bytes /
1_048_576`), matching [`Snapshot::ram_mb`] and [`Snapshot::vram_mb`]"*, and `format_total` and
`format_used` repeat the note. `hmn` itself renders MiB/GiB honestly and has `format_vram_precise`
specifically so displayed units round-trip back into `--min`.

[`src/report.rs`](../../src/report.rs) is the exception. `ram_delta_mb`
([:38](../../src/report.rs#L38)) and `vram_delta_mb` ([:46](../../src/report.rs#L46)) are documented
as *"in megabytes"* with no qualification, and `format_delta` ([:68](../../src/report.rs#L68)) and
`format_before_after` ([:88](../../src/report.rs#L88)) document and emit `MB` — including a `" /
{:.0} MB"` total computed inline at [:99](../../src/report.rs#L99) with its own `1_048_576.0`
divisor rather than going through `GpuDeviceInfo`. Five rendered `MB` labels, all MiB.

The module doc explains the naming: these are *"preserved verbatim from `candle-mi`'s in-tree memory
module"* so Phase 3 is a feature flip, not a rewrite. That is a good reason to keep
`ram_delta_mb`'s **name** and the **output text** exactly as they are. It is not a reason to leave
the **rustdoc** silent. **Fix:** add the same one-line "`MB` here means `MiB`" note the three
`GpuDeviceInfo` formatters already carry, to all four `MemoryReport` methods. Docs-only, no
behaviour change, no compatibility cost.

### 4.2 `src/bin/hmn.rs` is 2,466 production lines in one file, and growing

2,466 production + 2,040 inline test lines = 4,506 total, up from 3,618 at the last audit — a 25%
growth in two releases. That one file holds the clap definitions, six subcommand runners, roughly
twenty formatters, the `WatchState` machine, size/duration parsing, and the shared helpers. No
finding attaches to this on its own; the tests are thorough and clippy is clean. It is recorded
because §P3.4's three table renderers, §P2.1's four JSON spellings and §P2.2's doubled SPILL mapping
all live here, and because every one of v0.2.12's requests adds to this same file. A split along the
seams that already exist — `hmn/format.rs` for the pure formatters, `hmn/watch.rs` for the watch
loop and its state — would be mechanical and would give the formatter tests a natural home. Worth
considering *before* v0.2.12's additions, not after.

---

## Bearing on the pending v0.2.12 work

The dogfooding report this audit precedes asks for `--filter <SUBSTRING>` and `--min <SIZE>` on
`watch`, a criterion announcement on the `.err` header, and (as its one wire-format change) a
`{"kind":"start", ...}` record. Three of the findings above sit directly in that path:

1. **§P2.1 first.** A `start` record adds a **fifth** spelling of the JSON contract and a second
   record kind that consumers must tolerate. Land the four-way key-sequence parity test *before*
   writing the fifth emitter, and the new record arrives with a guard already in place. This is the
   single highest-value item in this audit relative to the work about to be done, and it is ~15
   lines of test.
2. **§P2.2 and the header line.** Request 3 wants the criterion echoed on the `.err` header, and
   `--filter` means the followed-set breadcrumb gains a reason a PID was *not* selected. Both touch
   the same "how do we render an honest can't-tell / why-not" vocabulary that §P2.2's doubled
   mapping already encodes twice. Extract `spill_cell()` while you are in those formatters.
3. **§P3.4 and §4.2.** `--filter` composing with `--top` does not add a table, but a criterion line
   and an unresolved-but-unmatched count do add output surface to the file that is already the
   crate's largest. If the file is going to be split, splitting it before the feature lands is
   cheaper than after.

Nothing in this audit blocks v0.2.12, and none of the refactors in §P3 is a prerequisite.

---

## What is demonstrably healthy (verified during this audit)

- **No copy-paste at any meaningful scale.** Zero duplicate groups at a 20-line window, zero at 14.
- All five gates green locally on Windows: fmt, clippy `--all-targets --all-features` under
  `-D warnings`, 289 tests passing, doc build under `-D warnings`, `--no-default-features` check.
- The crate's existing anti-drift helpers all do their job and several document *why* they exist:
  `column_width`, `json_escape`, `write_episodes_json` (with a parity test),
  `default_dedicated_threshold` (shared by both spill conditions), `select_top_n_pids` (shares
  `hmn ps`'s comparator with `watch`'s auto-selection "so the two orderings can't drift apart"),
  `format_spill_report_with_prefix` (one report renderer for `spill` and `watch`).
- NVML init/shutdown pairing re-verified balanced on every return path in all three entry points,
  with no `?` escaping between init and shutdown.
- 11 of the 15 items from the 2026-08-17 audit are fully closed, including every P1 and every P2.
- No dead or drifted duplicate was found: every cluster examined was in agreement with its twins at
  the time of audit.

---

## Suggested remediation order

| # | Item | Effort | Ships with |
|---|------|--------|------------|
| 1 | §P2.1 — four-way JSON key-sequence parity **test** (no production change) | ~15 lines | before v0.2.12's `start` record |
| 2 | §P2.2 — extract `spill_cell()`; §P3.5 — `name_suffix()` / `mib()` in `snapshot.rs` | ~30 min | v0.2.12 ride-along |
| 3 | §P1.1 option 1 — skip-on-bad-adapter in the four remaining DXGI walks | ~20 lines + review | v0.2.12 |
| 4 | Prior audit's 3.4, 3.5, 3.6 + §4.1 `MemoryReport` MiB note + the `tests/common` comment correction + the ROADMAP `spill_condition` rewording | ~30 min, docs-only | one `docs:` commit |
| 5 | §P3.1 — `run_smi()` helper; §P3.2 — `open_query()` helper | ~1 h total | v0.2.12 or v0.2.13 |
| 6 | §P2.1 — single `write_spill_report_fields()` replacing all four spellings | ~1 h | after item 1 |
| 7 | §4.2 — split `src/bin/hmn.rs` along its existing seams | half a day | own slot, before further growth |
| 8 | §P1.1 option 2 — one DXGI adapter iterator for all six walks | half a day + adversarial pass | own slot |
| 9 | §P3.3 — `NvmlSession` RAII guard; §P3.4 — internal `Table` renderer | a day each | own slots, gated on need |

Items 1–4 are additive or docs-only and carry no behaviour risk. Items 5–6 are behaviour-preserving
helper extractions covered by existing tests. Items 7–9 each touch either every `unsafe` block in a
file or the CLI's entire output contract, and each deserves the two-pass review the project already
applies to feature work.
