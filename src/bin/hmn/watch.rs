// SPDX-License-Identifier: MIT OR Apache-2.0

//! `hmn watch`: attach to running PIDs (or follow the top-N by committed
//! `VRAM`) and sample per-PID usage plus adapter spill state on a timer —
//! a scrolling `time(1)`-style sampler, not a TUI.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use hypomnesis::{GpuProcessEntry, SpillReport, SpillTracker, device_info, gpu_processes};

use crate::format::{
    Table, device_name_suffix, duration_ms, format_vram, iso8601_utc_millis, json_escape,
    spill_cell,
};
use crate::ps::{PsRow, SortKey, ps_row_comparator};
use crate::spill::{format_spill_report_with_prefix, write_spill_report_fields};

/// Minimum unresolved-PID cumulative growth (bytes, either committed or
/// shared) that triggers the one-shot "unresolved process grew" stderr
/// hint. Same magnitude as [`hypomnesis::spill`]'s
/// `DEFAULT_SHARED_GROWTH_BYTES` (256 MiB) — large enough to clear
/// ordinary counter jitter, small enough to fire well before a real
/// leak becomes a problem.
const UNRESOLVED_GROWTH_HINT_BYTES: u64 = 256 * 1024 * 1024;

/// How long a single sleep chunk in the watch loop lasts before
/// re-checking the Ctrl+C flag — keeps interrupt latency low even when
/// `--interval` is minutes long.
const WATCH_SLEEP_CHUNK: Duration = Duration::from_millis(200);

/// Per-watched-PID bookkeeping accumulated across the whole watch.
struct WatchedPidState {
    /// Committed bytes at the first sample — the baseline the closing
    /// summary reports growth against.
    baseline_used_bytes: u64,
    /// Shared-resident bytes at the first sample.
    baseline_shared_bytes: u64,
    /// Committed bytes at the previous sample — the per-interval delta
    /// is computed against this, then it is overwritten.
    prev_used_bytes: u64,
    /// Shared-resident bytes at the previous sample.
    prev_shared_bytes: u64,
    /// Highest committed reading seen across the watch.
    peak_used_bytes: u64,
    /// Highest shared-resident reading seen across the watch.
    peak_shared_bytes: u64,
    /// Name from the most recent sample that saw this PID; `None` once
    /// no sample has ever resolved one.
    last_name: Option<String>,
    /// Whether the unresolved-growth hint has already fired for this
    /// PID (fires at most once per watch).
    growth_hint_fired: bool,
}

impl WatchedPidState {
    /// Fresh state seeded from a PID's first sample: baseline, previous,
    /// and peak all start at the first reading, so the first interval's
    /// delta is `+0`.
    const fn new(used_bytes: u64, shared_bytes: u64) -> Self {
        Self {
            baseline_used_bytes: used_bytes,
            baseline_shared_bytes: shared_bytes,
            prev_used_bytes: used_bytes,
            prev_shared_bytes: shared_bytes,
            peak_used_bytes: used_bytes,
            peak_shared_bytes: shared_bytes,
            last_name: None,
            growth_hint_fired: false,
        }
    }
}

/// Accumulated per-PID watch state, plus the order PIDs were first seen
/// in.
///
/// A plain `HashMap<u32, WatchedPidState>` would lose insertion order —
/// fine when the watched PID set is fixed for the whole run (the
/// pre-`--follow-new` case, where the closing summary iterates the
/// original fixed list instead), but under `--follow-new` the closing
/// summary must instead walk *every* PID ever tracked (departed or
/// still active) in a deterministic, meaningful order. `seen_order`
/// records that order — chronological, first sighting — without pulling
/// in an ordered-map dependency: [`WatchState::track`] is the sole
/// insertion point and is the only place a PID is ever appended to it,
/// exactly once, the first time that PID is seen.
struct WatchState {
    /// Per-PID accumulated state, keyed by PID.
    by_pid: HashMap<u32, WatchedPidState>,
    /// PIDs in first-seen order. Never contains a duplicate — see
    /// [`WatchState::track`].
    seen_order: Vec<u32>,
}

impl WatchState {
    /// Fresh, empty state.
    fn new() -> Self {
        Self {
            by_pid: HashMap::new(),
            seen_order: Vec::new(),
        }
    }

    /// Get the existing entry for `pid`, or seed a fresh one from
    /// `(used_bytes, shared_bytes)` and record `pid` in
    /// [`Self::seen_order`] — but only on this, the *first* time `pid`
    /// is tracked. A PID that later drops out of the followed set and
    /// re-enters keeps its original `seen_order` position and its
    /// existing history (no reset; see [`process_sample`]'s doc comment
    /// for why re-entry is deliberately not treated as a new process).
    fn track(&mut self, pid: u32, used_bytes: u64, shared_bytes: u64) -> &mut WatchedPidState {
        // `self.seen_order` and `self.by_pid` are disjoint fields, so
        // borrowing the former up front and capturing it in the
        // `or_insert_with` closure below (which only runs, and so only
        // pushes, on an actual insertion) needs no unwrap/expect/entry
        // double-lookup to track first-seen order.
        let seen_order = &mut self.seen_order;
        self.by_pid.entry(pid).or_insert_with(|| {
            seen_order.push(pid);
            WatchedPidState::new(used_bytes, shared_bytes)
        })
    }
}

/// One PID's rendered row for one interval — output of [`process_sample`],
/// consumed by the text-table and JSONL formatters.
struct WatchSampleRow {
    /// Process ID.
    pid: u32,
    /// Process name from this sample; `None` renders as `?`.
    name: Option<String>,
    /// Committed bytes this sample.
    used_bytes: u64,
    /// Signed delta vs. the previous sample (negative = freed).
    used_delta: i64,
    /// Shared-resident bytes this sample.
    shared_bytes: u64,
    /// Signed delta vs. the previous sample.
    shared_delta: i64,
    /// Adapter-wide instantaneous spill state at this sample
    /// ([`SpillTracker::is_spilling`]) — the same value on every row
    /// sharing this interval's timestamp; spill is a device-level
    /// phenomenon, not a per-PID one. `None` when spill isn't
    /// measurable here (no tracker constructed, or
    /// `SpillTracker::is_measurable` is false for this instance —
    /// Linux/macOS, no `pdh` feature, no `GPU Adapter Memory` counter
    /// set) — never collapsed into `Some(false)`, matching `hmn ps`'s
    /// SPILL column/`spilling` field contract.
    spilling: Option<bool>,
}

/// End-of-watch peak/baseline summary for one watched PID.
pub struct WatchPidSummary {
    /// Process ID.
    pub pid: u32,
    /// Name from the most recent sample that resolved one.
    pub name: Option<String>,
    /// Committed bytes at the first sample.
    pub baseline_used_bytes: u64,
    /// Highest committed reading across the watch.
    pub peak_used_bytes: u64,
    /// Shared-resident bytes at the first sample.
    pub baseline_shared_bytes: u64,
    /// Highest shared-resident reading across the watch.
    pub peak_shared_bytes: u64,
}

/// Signed byte delta `current - previous`.
///
/// VRAM byte counts are far below `i64::MAX` (2^63) for any real GPU, so
/// the widening cast cannot lose information.
const fn signed_delta(current: u64, previous: u64) -> i64 {
    // CAST: u64 → i64, VRAM byte counts (< 2^53 in practice) fit
    // trivially; deltas can be negative (VRAM freed), which u64 cannot
    // represent.
    #[allow(clippy::as_conversions, clippy::cast_possible_wrap)]
    let (c, p) = (current as i64, previous as i64);
    c - p
}

/// Human-readable signed VRAM delta: `"+700 MiB"`, `"-1.2 GiB"`,
/// `"+0 B"` for an exact-zero delta (avoids a `"-0 …"` reading for
/// negative deltas that round to zero under [`format_vram`]'s MiB
/// granularity).
fn format_delta(bytes: i64) -> String {
    if bytes == 0 {
        return "+0 B".to_owned();
    }
    let sign = if bytes < 0 { '-' } else { '+' };
    format!("{sign}{}", format_vram(bytes.unsigned_abs()))
}

/// `u8` exit code conveying whether spill was observed during a watch:
/// `0` clean, `1` spill observed at least once. Hard-error paths (bad
/// device or nothing to auto-select, from [`run_watch`]; an invalid
/// argument combination, from [`Selection::new`] via `main`) return `2`
/// directly, bypassing this mapping.
const fn watch_exit_code(spilled: bool) -> u8 {
    if spilled { 1 } else { 0 }
}

/// Select the PIDs to watch when none were given explicitly: sort by
/// [`ps_row_comparator`] under [`SortKey::Dedicated`] — always the
/// dedicated-descending key, sharing `run_ps`'s exact comparator
/// (including its name/PID tie-break chain) so the two orderings can't
/// silently drift — and take the first `n`. Pure — the auto-selection
/// policy is unit-testable without any FFI.
fn select_top_n_pids(rows: &[PsRow], n: usize) -> Vec<u32> {
    let mut sorted: Vec<&PsRow> = rows.iter().collect();
    let cmp = ps_row_comparator(SortKey::Dedicated);
    sorted.sort_by(|a, b| cmp(a, b));
    sorted.into_iter().take(n).map(|r| r.pid).collect()
}

/// Sleep for `total`, checking `interrupted` every
/// [`WATCH_SLEEP_CHUNK`] so a Ctrl+C during a long `--interval` is
/// noticed promptly rather than only after the full sleep elapses.
fn sleep_interruptibly(total: Duration, interrupted: &AtomicBool) {
    let mut remaining = total;
    while remaining > Duration::ZERO && !interrupted.load(Ordering::Relaxed) {
        let step = remaining.min(WATCH_SLEEP_CHUNK);
        std::thread::sleep(step);
        remaining = remaining.saturating_sub(step);
    }
}

/// Filter out `name` values that don't represent a genuinely resolved
/// process identity for [`process_sample`]'s PID-reuse comparison:
/// `None` (unresolved) and the Windows-only `"[protected]"`/`"[exited]"`
/// synthetic brackets (still unresolved, just with more detail than a
/// bare `?`). `"[kernel]"` is deliberately *not* filtered — `PID 4` is
/// permanently the kernel and never flickers, so it is safe to treat as
/// a stable, comparable name.
#[must_use]
fn resolved_name(name: Option<&str>) -> Option<&str> {
    name.filter(|n| *n != "[protected]" && *n != "[exited]")
}

/// Fold one sample into `state`, observe the spill tracker, and return
/// one rendered [`WatchSampleRow`] per watched PID (in `watched`'s
/// order). A watched PID absent from `rows` renders as `0 B` / `0 B` for
/// this interval — `hmn watch` cannot distinguish "exited" from
/// "currently holds no GPU memory" and does not try to (see the `Watch`
/// subcommand's doc comment).
///
/// `watched` need not be the same slice across calls — under
/// `--follow-new` it's recomputed every interval — and a PID re-entering
/// `watched` after an absence resumes its existing [`WatchState`] entry
/// rather than starting fresh: [`WatchState::track`] only seeds a PID
/// once, the first time it's ever seen, by design (an OS process
/// legitimately dipping below rank `--top` for one interval and
/// recovering is not a new process; the name-change reset below is the
/// intentionally narrower signal for genuine OS PID reuse).
///
/// Emits the one-shot `?`-row growth hint to stderr the first interval
/// an unresolved watched PID's cumulative growth crosses
/// [`UNRESOLVED_GROWTH_HINT_BYTES`] — "unresolved" here means `name` is
/// `None` or (Windows-only, since v0.2.8) the `"[protected]"` bracket;
/// `"[exited]"` does not count, since a process already confirmed gone
/// cannot meaningfully "grow". Also detects a watched PID being recycled
/// by the OS mid-watch (its [`resolved_name`] changes between samples)
/// and resets that PID's baseline/peak so deltas describe the new
/// process rather than mixing two processes' readings — best-effort
/// (unresolved-name churn can't be distinguished from reuse).
fn process_sample(
    rows: &[GpuProcessEntry],
    state: &mut WatchState,
    watched: &[u32],
    elapsed: Duration,
    tracker: Option<&mut SpillTracker>,
) -> Vec<WatchSampleRow> {
    // `None` — not `Some(false)` — when there's no way to tell:
    // `tracker: None` (construction failed) or this instance isn't
    // measurable (`SpillTracker::is_measurable`). Matches `hmn ps`'s
    // `spilling` field contract; a plain `is_some_and` here would
    // collapse "can't tell" into "measured, not spilling", which is
    // exactly the misreading that field's honesty promise exists to
    // prevent.
    let spilling: Option<bool> = tracker.and_then(|t| {
        t.observe(format!("+{:.1}s", elapsed.as_secs_f64()));
        t.is_measurable().then(|| t.is_spilling())
    });

    let mut out = Vec::with_capacity(watched.len());
    for &pid in watched {
        let found = rows.iter().find(|r| r.pid == pid);
        let (name, used_bytes, shared_bytes) = found.map_or((None, 0, 0), |r| {
            (r.name.clone(), r.used_bytes, r.shared_used_bytes)
        });

        let entry = state.track(pid, used_bytes, shared_bytes);

        // The OS can recycle a PID mid-watch: a resolved name that
        // changes between samples is the only signal `hmn watch` has
        // that "pid" now names a different process than the one it
        // baselined against. Treat it as a fresh attach — reset
        // baseline/peak/prev to this sample so the closing summary and
        // this row's delta describe the *new* process, not a mix of
        // both. `None` on either side (still unresolved, or a
        // transient resolution race) is not treated as a change — nor
        // is the Windows-only `"[protected]"`/`"[exited]"` synthetic
        // brackets, which can flicker in and out for one interval (e.g.
        // a transient `Toolhelp32Snapshot` failure) without the
        // underlying process actually changing — see `resolved_name`
        // and the `last_name` update below, both of which stay sticky
        // across an unresolved sample.
        if let (Some(old), Some(new)) = (
            resolved_name(entry.last_name.as_deref()),
            resolved_name(name.as_deref()),
        ) && old != new
        {
            entry.baseline_used_bytes = used_bytes;
            entry.baseline_shared_bytes = shared_bytes;
            entry.prev_used_bytes = used_bytes;
            entry.prev_shared_bytes = shared_bytes;
            entry.peak_used_bytes = used_bytes;
            entry.peak_shared_bytes = shared_bytes;
            entry.growth_hint_fired = false;
            eprintln!(
                "hmn watch: pid={pid} name changed ({old} → {new}) — likely PID reuse by the OS; baseline reset"
            );
        }

        let used_delta = signed_delta(used_bytes, entry.prev_used_bytes);
        let shared_delta = signed_delta(shared_bytes, entry.prev_shared_bytes);
        entry.prev_used_bytes = used_bytes;
        entry.prev_shared_bytes = shared_bytes;
        entry.peak_used_bytes = entry.peak_used_bytes.max(used_bytes);
        entry.peak_shared_bytes = entry.peak_shared_bytes.max(shared_bytes);
        entry.last_name = resolved_name(name.as_deref())
            .map(ToOwned::to_owned)
            .or_else(|| entry.last_name.clone());

        let still_unresolved = name.is_none() || name.as_deref() == Some("[protected]");
        if !entry.growth_hint_fired && still_unresolved {
            let grown = used_bytes
                .saturating_sub(entry.baseline_used_bytes)
                .max(shared_bytes.saturating_sub(entry.baseline_shared_bytes));
            if grown >= UNRESOLVED_GROWTH_HINT_BYTES {
                entry.growth_hint_fired = true;
                eprintln!(
                    "hmn watch: unresolved pid={pid} grew +{} since attach — re-run elevated to identify",
                    format_vram(grown)
                );
            }
        }

        out.push(WatchSampleRow {
            pid,
            name,
            used_bytes,
            used_delta,
            shared_bytes,
            shared_delta,
            spilling,
        });
    }
    out
}

/// Format one interval's rows as a text table (no header — the caller
/// prints the column header once up front). Column widths are computed
/// per call from that interval's own cells, like `ps::format_ps_table`;
/// consecutive intervals may re-align slightly as values change width,
/// an acceptable trade-off for a continuously-appended stream (`--json`
/// is the stable-shape option for scripts).
fn format_watch_rows_text(elapsed: Duration, rows: &[WatchSampleRow]) -> String {
    let mut table = Table::new(&[
        "PID",
        "NAME",
        "COMMITTED",
        "\u{394}COMMIT",
        "SHARED",
        "\u{394}SHARED",
        "SPILL",
    ]);
    for r in rows {
        table.push_row(vec![
            r.pid.to_string(),
            // BORROW: explicit to_owned — the table owns its cells; "?" is
            // the "can't tell" glyph for an unresolved name.
            r.name.as_deref().unwrap_or("?").to_owned(),
            format_vram(r.used_bytes),
            format_delta(r.used_delta),
            format_vram(r.shared_bytes),
            format_delta(r.shared_delta),
            // BORROW: explicit to_owned — the table owns its cells.
            spill_cell(r.spilling).to_owned(),
        ]);
    }
    let time_label = format!("+{:.1}s", elapsed.as_secs_f64());
    table.render(None, &format!("{time_label:<8}  "))
}

/// Format the watch column header line (text mode), printed once before
/// the loop starts.
fn format_watch_header_text() -> String {
    format!(
        "{:<8}  {:<6}  {:<12}  {:<9}  {:<9}  {:<9}  {:<9}  {:<5}\n",
        "TIME", "PID", "NAME", "COMMITTED", "\u{394}COMMIT", "SHARED", "\u{394}SHARED", "SPILL"
    )
}

/// Format one interval's rows as JSON Lines: one `"kind":"sample"`
/// object per row, newline-terminated, ready to pipe to `jq -c`.
/// `wall_clock` (captured at the same instant as `t_ms`'s `elapsed`,
/// just before this interval's `gpu_processes()` call — not after,
/// so the two timestamps in one sample never straddle the query's own
/// duration) is the same for every row in the interval — formatted
/// once via [`iso8601_utc_millis`], not per row.
fn format_watch_rows_json(
    elapsed: Duration,
    wall_clock: SystemTime,
    rows: &[WatchSampleRow],
) -> String {
    let mut out = String::new();
    let wall_clock_json = iso8601_utc_millis(wall_clock);
    for row in rows {
        let name_json = row.name.as_deref().map_or_else(
            || String::from("null"),
            |n| format!("\"{}\"", json_escape(n)),
        );
        let spilling_json = match row.spilling {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        };
        let _ = writeln!(
            out,
            r#"{{"kind":"sample","t_ms":{},"wall_clock":"{wall_clock_json}","pid":{},"name":{name_json},"used_bytes":{},"used_delta_bytes":{},"shared_used_bytes":{},"shared_delta_bytes":{},"spilling":{spilling_json}}}"#,
            duration_ms(elapsed),
            row.pid,
            row.used_bytes,
            row.used_delta,
            row.shared_bytes,
            row.shared_delta,
        );
    }
    out
}

/// Format the end-of-watch per-PID peak/baseline block (text mode).
/// Empty `per_pid` renders as an empty string (nothing to show). Data
/// rows are indented to sit under the header's columns, past its
/// `hmn watch: per-PID` lead-in.
fn format_watch_per_pid_block(per_pid: &[WatchPidSummary]) -> String {
    if per_pid.is_empty() {
        return String::new();
    }
    let mut table = Table::new(&[
        "PID",
        "NAME",
        "BASELINE COMMIT",
        "PEAK COMMIT",
        "BASELINE SHARED",
        "PEAK SHARED",
    ]);
    for p in per_pid {
        table.push_row(vec![
            p.pid.to_string(),
            // BORROW: explicit to_owned — the table owns its cells; "?" is
            // the "can't tell" glyph for an unresolved name.
            p.name.as_deref().unwrap_or("?").to_owned(),
            format_vram(p.baseline_used_bytes),
            format_vram(p.peak_used_bytes),
            format_vram(p.baseline_shared_bytes),
            format_vram(p.peak_shared_bytes),
        ]);
    }
    let header_prefix = "hmn watch: per-PID  ";
    table.render(Some(header_prefix), &" ".repeat(header_prefix.len()))
}

/// Format the closing summary in text mode: the adapter-level report
/// (via [`format_spill_report_with_prefix`] under the `hmn watch`
/// prefix) or, when spill tracking was unavailable for this run, a
/// one-line notice — followed either way by the per-PID block.
fn format_watch_summary_text(report: Option<&SpillReport>, per_pid: &[WatchPidSummary]) -> String {
    let mut out = report.map_or_else(
        || "hmn watch: spill tracking unavailable for this run; per-PID VRAM below\n".to_owned(),
        |r| format_spill_report_with_prefix("hmn watch", r),
    );
    out.push_str(&format_watch_per_pid_block(per_pid));
    out
}

/// Format the closing summary as one JSON object:
/// `{"kind":"summary",...adapter SpillReport fields...,"per_pid":[...]}`.
/// `report: None` (spill tracking unavailable) emits the same
/// all-zeros `"measurable":false` shape `hmn spill --json` uses — both
/// go through [`write_spill_report_fields`] — so scripted consumers
/// always parse one shape either way.
pub fn format_watch_summary_json(
    report: Option<&SpillReport>,
    per_pid: &[WatchPidSummary],
) -> String {
    let mut out = String::from(r#"{"kind":"summary","#);
    write_spill_report_fields(&mut out, report);
    out.push_str(r#","per_pid":["#);
    for (i, p) in per_pid.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let name_json = p.name.as_deref().map_or_else(
            || String::from("null"),
            |n| format!("\"{}\"", json_escape(n)),
        );
        let _ = write!(
            out,
            r#"{{"pid":{},"name":{name_json},"baseline_used_bytes":{},"peak_used_bytes":{},"baseline_shared_bytes":{},"peak_shared_bytes":{}}}"#,
            p.pid,
            p.baseline_used_bytes,
            p.peak_used_bytes,
            p.baseline_shared_bytes,
            p.peak_shared_bytes,
        );
    }
    out.push_str("]}\n");
    out
}

/// How `hmn watch` chooses the PIDs it follows.
///
/// The one value the selection itself ([`Self::select`]) and the stderr
/// header's description of it ([`Self::describe`]) are both derived
/// from, so what a capture *says* it selected and what it actually
/// selected cannot drift apart. Built by [`Self::new`], which rejects
/// argument combinations that only make sense for auto-selection before
/// any hardware is touched.
pub struct Selection {
    /// Explicit PIDs from the command line, deduplicated, in the order
    /// given. Non-empty means explicit mode: exactly these are watched.
    explicit: Vec<u32>,
    /// How many processes auto-selection keeps (`--top`). Unused in
    /// explicit mode.
    top: usize,
    /// Whether auto-selection re-runs every interval (`--follow-new`)
    /// rather than once at attach.
    follow_new: bool,
    /// `--filter` patterns, as typed. Empty means no name filter; a name
    /// qualifies when it contains any one of them, ignoring case.
    filters: Vec<String>,
}

/// The outcome of one [`Selection::select`] call.
struct Selected {
    /// The PIDs to watch, in selection order.
    pids: Vec<u32>,
    /// PIDs `--filter` could not judge: no resolvable name in this sample
    /// and none remembered from an earlier one. Always empty without
    /// `--filter`.
    unmatchable: Vec<u32>,
}

impl Selection {
    /// Build the selection from `hmn watch`'s parsed arguments.
    ///
    /// # Errors
    ///
    /// Returns the user-facing message (without the `hmn: ` prefix) when
    /// `--follow-new` or `--filter` is combined with explicit PIDs: both
    /// narrow auto-selection, and there is no top-N to narrow for a fixed
    /// list.
    pub fn new(
        pids: &[u32],
        top: usize,
        follow_new: bool,
        filters: &[String],
    ) -> Result<Self, String> {
        let mut seen = HashSet::new();
        let explicit: Vec<u32> = pids.iter().copied().filter(|p| seen.insert(*p)).collect();
        if !explicit.is_empty() {
            if follow_new {
                return Err(
                    "watch --follow-new only applies to auto-selection; drop --follow-new \
                     or the explicit PID list"
                        .to_owned(),
                );
            }
            if !filters.is_empty() {
                return Err(
                    "watch --filter only applies to auto-selection; drop --filter or the \
                     explicit PID list"
                        .to_owned(),
                );
            }
        }
        Ok(Self {
            explicit,
            top,
            follow_new,
            filters: filters.to_vec(),
        })
    }

    /// Resolve which PIDs to watch from one sample: the explicit PIDs
    /// unchanged in explicit mode (always watched exactly as given);
    /// otherwise the processes passing `--filter` (all of them without
    /// it), then the top `top` of those by committed VRAM — sharing
    /// `hmn ps`'s own comparator via [`select_top_n_pids`], so the two
    /// orderings cannot drift apart.
    ///
    /// Under `--filter`, a row is judged by [`matchable_name`]: its
    /// current resolved name, else the name `state` last resolved for that
    /// PID. A row with neither is reported in [`Selected::unmatchable`]
    /// instead of being dropped silently.
    ///
    /// Called once before the watch loop always, and again every interval
    /// under `--follow-new` (cheap: `rows` numbers in the tens, and
    /// `--interval` is 5s+ apart by default).
    #[must_use]
    fn select(&self, rows: &[GpuProcessEntry], state: &WatchState) -> Selected {
        if !self.explicit.is_empty() {
            return Selected {
                pids: self.explicit.clone(),
                unmatchable: Vec::new(),
            };
        }
        let mut unmatchable = Vec::new();
        // device_index / device_name / spilling are unused by
        // SortKey::Dedicated's comparator (pid / used_bytes / name only) —
        // defaulted rather than threaded through from the caller, which has
        // no device-name or live-spill context of its own to give.
        let ps_rows: Vec<PsRow> = rows
            .iter()
            .filter(|e| {
                if self.filters.is_empty() {
                    return true;
                }
                let sticky = state
                    .by_pid
                    .get(&e.pid)
                    .and_then(|s| s.last_name.as_deref());
                matchable_name(e.name.as_deref(), sticky).map_or_else(
                    || {
                        unmatchable.push(e.pid);
                        false
                    },
                    |name| matches_any(name, &self.filters),
                )
            })
            .map(|e| PsRow {
                pid: e.pid,
                // BORROW: clone — e is borrowed from `rows`.
                name: e.name.clone(),
                used_bytes: e.used_bytes,
                shared_used_bytes: e.shared_used_bytes,
                device_index: 0,
                device_name: None,
                spilling: None,
            })
            .collect();
        Selected {
            pids: select_top_n_pids(&ps_rows, self.top),
            unmatchable,
        }
    }

    /// The stderr header's mode clause, given how many PIDs were
    /// selected at attach — e.g. `following top 3 by committed among
    /// names containing "train" (case-insensitive) (re-selected every
    /// interval), 1 initially`. Without `--filter`, byte-identical to the
    /// clauses `hmn watch` printed before it existed.
    #[must_use]
    fn describe(&self, initially: usize) -> String {
        let top = self.top;
        let criterion = self.criterion();
        if self.follow_new {
            format!(
                "following top {top} by committed{criterion} (re-selected every interval), \
                 {initially} initially"
            )
        } else if self.explicit.is_empty() {
            format!("watching {initially} PID(s) (top {top} by committed{criterion})")
        } else {
            format!("watching {initially} PID(s)")
        }
    }

    /// The auto-selection criterion beyond "top N by committed", as a
    /// clause to splice after it (leading space included), or the empty
    /// string when there is none — e.g. ` among names containing "a" or
    /// "b" (case-insensitive)`. Patterns are `Debug`-quoted, so one
    /// containing a quote or a space reads unambiguously.
    #[must_use]
    fn criterion(&self) -> String {
        if self.filters.is_empty() {
            return String::new();
        }
        let patterns: Vec<String> = self.filters.iter().map(|f| format!("{f:?}")).collect();
        format!(
            " among names containing {} (case-insensitive)",
            patterns.join(" or ")
        )
    }
}

/// The name `--filter` judges a row by: its current name if that is a
/// genuinely resolved one, else `sticky` — the name this PID last resolved
/// to in an earlier sample, so a followed process whose name flickers to
/// `[protected]` for one interval is not evicted by it. `None` when
/// neither is available. Besides what [`resolved_name`] already excludes,
/// the `nvidia-smi` fallback's literal `?` is not a name either.
#[must_use]
fn matchable_name<'a>(current: Option<&'a str>, sticky: Option<&'a str>) -> Option<&'a str> {
    let real = |n: &&str| *n != "?";
    resolved_name(current)
        .filter(real)
        .or_else(|| resolved_name(sticky).filter(real))
}

/// Whether `name` contains any of `patterns`, ignoring case (`--filter`'s
/// matching rule).
#[must_use]
fn matches_any(name: &str, patterns: &[String]) -> bool {
    let name = name.to_lowercase();
    patterns.iter().any(|p| name.contains(&p.to_lowercase()))
}

/// The one-shot stderr notices for PIDs in `unmatchable` that this watch
/// has not announced yet, recording them in `announced` — so a process
/// `--filter` cannot judge is named exactly once rather than every
/// interval, and never passes silently.
#[must_use]
fn unmatchable_notices(unmatchable: &[u32], announced: &mut HashSet<u32>) -> Vec<String> {
    unmatchable
        .iter()
        .filter(|pid| announced.insert(**pid))
        .map(|pid| format!("hmn watch: pid={pid} has no resolvable name; --filter cannot match it"))
        .collect()
}

/// Parse one `--filter` pattern: any non-blank string, kept as typed. A
/// blank pattern would match every name — almost certainly a scripting
/// mistake (an unset variable), so it is rejected rather than accepted as
/// a silent no-op.
///
/// # Errors
///
/// Returns an error message when `s` is empty or whitespace only.
pub fn parse_filter_pattern(s: &str) -> std::result::Result<String, String> {
    if s.trim().is_empty() {
        return Err(format!(
            "invalid filter {s:?}: expected a non-blank name pattern"
        ));
    }
    // BORROW: explicit to_owned — clap stores the parsed value.
    Ok(s.to_owned())
}

/// Build the stderr breadcrumb naming PIDs that entered or left the
/// followed set between two consecutive `--follow-new` intervals, or
/// `None` when the set didn't change. Entered-PID names come from the
/// current sample's `rows`; left-PID names come from `state`'s last
/// known name for that PID (it is already absent from `rows`, by
/// definition of having left). Purely cosmetic: doesn't affect the
/// JSONL stream shape or the closing summary.
///
/// `prev_watched` must be captured *before* the caller reassigns its
/// `watched` variable to the freshly [`Selection::select`]-computed
/// set — the two arguments have to actually differ for the diff to mean
/// anything. Call order relative to [`process_sample`] does not matter
/// on its own: `process_sample` only touches a PID present in the
/// `watched` slice it is given, so a departed PID's [`WatchState`] entry
/// is untouched that interval regardless of when this function runs
/// relative to it.
fn format_followed_set_change(
    prev_watched: &[u32],
    new_watched: &[u32],
    rows: &[GpuProcessEntry],
    state: &WatchState,
    elapsed: Duration,
) -> Option<String> {
    let entered: Vec<u32> = new_watched
        .iter()
        .copied()
        .filter(|p| !prev_watched.contains(p))
        .collect();
    let left: Vec<u32> = prev_watched
        .iter()
        .copied()
        .filter(|p| !new_watched.contains(p))
        .collect();
    if entered.is_empty() && left.is_empty() {
        return None;
    }

    let name_for = |pid: u32| -> String {
        let name = rows
            .iter()
            .find(|r| r.pid == pid)
            .and_then(|r| r.name.clone())
            .or_else(|| state.by_pid.get(&pid).and_then(|s| s.last_name.clone()));
        name.map_or_else(|| format!("pid={pid}"), |n| format!("pid={pid} ({n})"))
    };

    let mut clauses = Vec::new();
    if !entered.is_empty() {
        let list = entered
            .into_iter()
            .map(name_for)
            .collect::<Vec<_>>()
            .join(", ");
        clauses.push(format!("entered {list}"));
    }
    if !left.is_empty() {
        let list = left
            .into_iter()
            .map(name_for)
            .collect::<Vec<_>>()
            .join(", ");
        clauses.push(format!("left {list}"));
    }
    Some(format!(
        "hmn watch: +{:.1}s followed set changed: {}",
        elapsed.as_secs_f64(),
        clauses.join("; ")
    ))
}

/// Run the `watch` subcommand: resolve the watched PID set, sample it on
/// a timer against [`SpillTracker`] + [`gpu_processes`] until
/// `--duration` elapses or Ctrl+C, then print the closing summary.
///
/// Returns `2` immediately on a hard error (device unreachable, or
/// nothing to auto-select without `--follow-new`); otherwise runs to
/// completion and returns [`watch_exit_code`] of whether spill was ever
/// observed. Invalid argument combinations never reach this function:
/// [`Selection::new`] rejects them in `main`'s dispatch, before any
/// backend call.
pub fn run_watch(
    selection: &Selection,
    interval: Duration,
    duration: Option<Duration>,
    device: u32,
    json: bool,
) -> std::process::ExitCode {
    let device_name = device_info(device).ok().and_then(|d| d.name);

    // `start` (the origin every later t_ms is measured from) and
    // first_wall_clock are captured together, right before the first
    // query — not after the Selection::select/SpillTracker::new/
    // ctrlc::set_handler setup below, which on Windows includes a real
    // DXGI walk plus GPU Adapter Memory PDH enumeration and can take
    // long enough to be visible. Capturing `start` later while
    // first_wall_clock stayed early would make the very first sample's
    // wall_clock predate t_ms's own zero point — the first row
    // reconstructed as `first_wall_clock + t_ms` would land earlier
    // than it actually happened, and every later row's gap from it
    // would read larger than `--interval`.
    let start = std::time::Instant::now();
    let first_wall_clock = SystemTime::now();
    let first_rows = match gpu_processes(device) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("hmn: watch failed to query device {device}: {e}");
            return std::process::ExitCode::from(2);
        }
    };

    let mut state = WatchState::new();
    // PIDs already named as unmatchable by `--filter`, so each is announced once.
    let mut announced = HashSet::new();
    let first = selection.select(&first_rows, &state);
    let mut watched = first.pids;
    if watched.is_empty() {
        let top = selection.top;
        let criterion = selection.criterion();
        if selection.follow_new {
            eprintln!(
                "hmn: watch found no GPU processes on device {device} yet (top {top} by \
                 committed{criterion}); waiting for work to appear"
            );
        } else {
            eprintln!(
                "hmn: watch found no GPU processes on device {device} to auto-select \
                 (top {top}{criterion}); re-run with an explicit PID once a workload is running"
            );
            return std::process::ExitCode::from(2);
        }
    }

    let mut tracker = match SpillTracker::new(device) {
        Ok(t) => Some(t),
        Err(e) => {
            eprintln!("hmn: watch spill tracking unavailable ({e}); showing per-PID VRAM only");
            None
        }
    };

    let interrupted = Arc::new(AtomicBool::new(false));
    {
        let interrupted = Arc::clone(&interrupted);
        if let Err(e) = ctrlc::set_handler(move || interrupted.store(true, Ordering::SeqCst)) {
            eprintln!(
                "hmn: watch failed to install Ctrl+C handler ({e}); interrupting will skip the closing summary"
            );
        }
    }

    eprintln!(
        "hmn watch: device {device}{}, interval {:.1}s, {}",
        device_name_suffix(device_name.as_deref()),
        interval.as_secs_f64(),
        selection.describe(watched.len()),
    );
    if !json {
        print!("{}", format_watch_header_text());
    }
    for notice in unmatchable_notices(&first.unmatchable, &mut announced) {
        eprintln!("{notice}");
    }

    let rows0 = process_sample(
        &first_rows,
        &mut state,
        &watched,
        Duration::ZERO,
        tracker.as_mut(),
    );
    if json {
        print!(
            "{}",
            format_watch_rows_json(Duration::ZERO, first_wall_clock, &rows0)
        );
    } else {
        print!("{}", format_watch_rows_text(Duration::ZERO, &rows0));
    }

    'watch: loop {
        if duration.is_some_and(|d| start.elapsed() >= d) || interrupted.load(Ordering::Relaxed) {
            break 'watch;
        }
        sleep_interruptibly(interval, &interrupted);
        if interrupted.load(Ordering::Relaxed) {
            break 'watch;
        }
        if duration.is_some_and(|d| start.elapsed() >= d) {
            break 'watch;
        }

        // Captured together, both right before the query, so t_ms and
        // wall_clock in the emitted sample refer to the same instant
        // rather than straddling gpu_processes()'s (non-zero, under
        // load) call duration.
        let elapsed = start.elapsed();
        let wall_clock = SystemTime::now();
        let rows = match gpu_processes(device) {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!(
                    "hmn watch: sample failed at +{:.1}s ({e}); skipping interval",
                    elapsed.as_secs_f64()
                );
                continue 'watch;
            }
        };

        if selection.follow_new {
            let next = selection.select(&rows, &state);
            for notice in unmatchable_notices(&next.unmatchable, &mut announced) {
                eprintln!("{notice}");
            }
            if let Some(msg) =
                format_followed_set_change(&watched, &next.pids, &rows, &state, elapsed)
            {
                eprintln!("{msg}");
            }
            watched = next.pids;
        }

        let sample = process_sample(&rows, &mut state, &watched, elapsed, tracker.as_mut());
        if json {
            print!("{}", format_watch_rows_json(elapsed, wall_clock, &sample));
        } else {
            print!("{}", format_watch_rows_text(elapsed, &sample));
        }
    }

    let report = tracker.map(SpillTracker::into_report);
    let per_pid: Vec<WatchPidSummary> = state
        .seen_order
        .iter()
        .map(|&pid| {
            let s = state.by_pid.get(&pid);
            WatchPidSummary {
                pid,
                name: s.and_then(|s| s.last_name.clone()),
                baseline_used_bytes: s.map_or(0, |s| s.baseline_used_bytes),
                peak_used_bytes: s.map_or(0, |s| s.peak_used_bytes),
                baseline_shared_bytes: s.map_or(0, |s| s.baseline_shared_bytes),
                peak_shared_bytes: s.map_or(0, |s| s.peak_shared_bytes),
            }
        })
        .collect();

    if json {
        print!("{}", format_watch_summary_json(report.as_ref(), &per_pid));
    } else {
        print!("{}", format_watch_summary_text(report.as_ref(), &per_pid));
    }

    std::process::ExitCode::from(watch_exit_code(
        report.as_ref().is_some_and(SpillReport::spilled),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[cfg(feature = "test-helpers")]
    use crate::test_support::spilling_report;
    use crate::test_support::{pid_summary, row};

    // --- format_delta ---

    #[test]
    fn format_delta_zero_is_plus_zero_bytes() {
        assert_eq!(format_delta(0), "+0 B");
    }

    #[test]
    fn format_delta_positive_mib() {
        assert_eq!(format_delta(700 * 1024 * 1024), "+700 MiB");
    }

    #[test]
    fn format_delta_negative_gib() {
        let one_point_two_gib = -(1024_i64 * 1024 * 1024 + 1024 * 1024 * 1024 / 5);
        assert_eq!(format_delta(one_point_two_gib), "-1.2 GiB");
    }

    // --- signed_delta ---

    #[test]
    fn signed_delta_basic() {
        assert_eq!(signed_delta(100, 40), 60);
        assert_eq!(signed_delta(40, 100), -60);
        assert_eq!(signed_delta(0, 0), 0);
    }

    // --- watch_exit_code ---

    #[test]
    fn watch_exit_code_clean_and_spilled() {
        assert_eq!(watch_exit_code(false), 0);
        assert_eq!(watch_exit_code(true), 1);
    }

    // --- select_top_n_pids ---

    #[test]
    fn select_top_n_pids_orders_by_committed_descending() {
        let rows = vec![
            row(1, Some("a.exe"), 1_000, 0, None),
            row(2, Some("b.exe"), 5_000, 0, None),
            row(3, Some("c.exe"), 3_000, 0, None),
        ];
        assert_eq!(select_top_n_pids(&rows, 2), vec![2, 3]);
    }

    #[test]
    fn select_top_n_pids_n_larger_than_rows_returns_all() {
        let rows = vec![row(1, Some("a.exe"), 1_000, 0, None)];
        assert_eq!(select_top_n_pids(&rows, 5), vec![1]);
    }

    #[test]
    fn select_top_n_pids_empty_rows() {
        assert!(select_top_n_pids(&[], 5).is_empty());
    }

    #[test]
    fn select_top_n_pids_ties_break_by_pid_ascending() {
        let rows = vec![
            row(20, Some("b.exe"), 1_000, 0, None),
            row(10, Some("a.exe"), 1_000, 0, None),
        ];
        assert_eq!(select_top_n_pids(&rows, 2), vec![10, 20]);
    }

    #[test]
    fn select_top_n_pids_ties_break_by_name_before_pid() {
        // Shares ps_row_comparator with `hmn ps`: a tie on used_bytes
        // breaks by name first, PID only as the final fallback. Name
        // and PID order deliberately *disagree* here (the lower PID, 1,
        // carries the alphabetically-later name) so this fixture can
        // actually distinguish "name-then-PID" from the old "PID-only"
        // rule: a PID-only tie-break would produce `[1, 99]`; the real
        // (name-first) comparator produces `[99, 1]`.
        let rows = vec![
            row(1, Some("z.exe"), 1_000, 0, None),
            row(99, Some("a.exe"), 1_000, 0, None),
        ];
        assert_eq!(select_top_n_pids(&rows, 2), vec![99, 1]);
    }

    // --- format_watch_rows_text / format_watch_header_text ---

    fn watch_row(
        pid: u32,
        name: Option<&str>,
        used: u64,
        used_delta: i64,
        shared: u64,
        shared_delta: i64,
        spilling: bool,
    ) -> WatchSampleRow {
        watch_row_opt(
            pid,
            name,
            used,
            used_delta,
            shared,
            shared_delta,
            Some(spilling),
        )
    }

    /// Like [`watch_row`] but with an explicit `Option<bool>` — for the
    /// "spill not measurable here" (`None`) cases `watch_row`'s plain
    /// `bool` can't express.
    fn watch_row_opt(
        pid: u32,
        name: Option<&str>,
        used: u64,
        used_delta: i64,
        shared: u64,
        shared_delta: i64,
        spilling: Option<bool>,
    ) -> WatchSampleRow {
        WatchSampleRow {
            pid,
            name: name.map(str::to_owned),
            used_bytes: used,
            used_delta,
            shared_bytes: shared,
            shared_delta,
            spilling,
        }
    }

    #[test]
    fn format_watch_header_text_has_expected_columns() {
        let h = format_watch_header_text();
        assert!(h.contains("TIME"));
        assert!(h.contains("PID"));
        assert!(h.contains("NAME"));
        assert!(h.contains("COMMITTED"));
        assert!(h.contains("SHARED"));
        assert!(h.contains("SPILL"));
    }

    #[test]
    fn format_watch_rows_text_single_row() {
        let r = watch_row(
            12345,
            Some("python.exe"),
            8 * 1024 * 1024 * 1024,
            0,
            142 * 1024 * 1024,
            0,
            false,
        );
        let s = format_watch_rows_text(Duration::from_secs(5), &[r]);
        assert!(s.starts_with("+5.0s"));
        assert!(s.contains("12345"));
        assert!(s.contains("python.exe"));
        assert!(s.contains("8.0 GiB"));
        assert!(s.contains("142 MiB"));
        assert!(s.contains("+0 B"));
        assert!(s.contains("no"));
    }

    #[test]
    fn format_watch_rows_text_spilling_row() {
        let r = watch_row(
            12345,
            Some("python.exe"),
            16 * 1024 * 1024 * 1024,
            700 * 1024 * 1024,
            2 * 1024 * 1024 * 1024,
            576 * 1024 * 1024,
            true,
        );
        let s = format_watch_rows_text(Duration::from_secs(10), &[r]);
        assert!(s.contains("+700 MiB"));
        assert!(s.contains("+576 MiB"));
        assert!(s.contains("SPILL"));
    }

    #[test]
    fn format_watch_rows_text_unmeasurable_spill_renders_question_mark_not_no() {
        // None (no tracker / not measurable on this platform) must
        // render distinctly from Some(false) ("no") — same "?, never
        // no" convention `hmn ps`'s SPILL column uses.
        let r = watch_row_opt(1, Some("py.exe"), 0, 0, 0, 0, None);
        let s = format_watch_rows_text(Duration::ZERO, &[r]);
        assert!(s.contains('?'));
        assert!(!s.contains("no"));
    }

    #[test]
    fn format_watch_rows_text_missing_name_renders_question_mark() {
        let r = watch_row(99, None, 0, 0, 0, 0, false);
        let s = format_watch_rows_text(Duration::ZERO, &[r]);
        assert!(s.contains("99"));
        assert!(s.contains('?'));
    }

    // --- format_watch_rows_json ---

    #[test]
    fn format_watch_rows_json_shape() {
        let r = watch_row(
            7,
            Some("py.exe"),
            1_048_576,
            1_048_576,
            424_242,
            -1_000,
            true,
        );
        let s = format_watch_rows_json(Duration::from_millis(3_500), SystemTime::UNIX_EPOCH, &[r]);
        assert!(s.starts_with(
            r#"{"kind":"sample","t_ms":3500,"wall_clock":"1970-01-01T00:00:00.000Z","pid":7,"name":"py.exe","used_bytes":1048576,"used_delta_bytes":1048576,"shared_used_bytes":424242,"shared_delta_bytes":-1000,"spilling":true}"#
        ));
        assert!(s.ends_with('\n'));
    }

    #[test]
    fn format_watch_rows_json_null_name() {
        let r = watch_row(7, None, 0, 0, 0, 0, false);
        let s = format_watch_rows_json(Duration::ZERO, SystemTime::UNIX_EPOCH, &[r]);
        assert!(s.contains(r#""name":null,"#));
    }

    #[test]
    fn format_watch_rows_json_unmeasurable_spilling_is_null_not_false() {
        let r = watch_row_opt(7, Some("py.exe"), 0, 0, 0, 0, None);
        let s = format_watch_rows_json(Duration::ZERO, SystemTime::UNIX_EPOCH, &[r]);
        assert!(s.contains(r#""spilling":null"#));
    }

    #[test]
    fn format_watch_rows_json_multiple_rows_multiple_lines() {
        let rows = vec![
            watch_row(1, Some("a.exe"), 0, 0, 0, 0, false),
            watch_row(2, Some("b.exe"), 0, 0, 0, 0, false),
        ];
        let s = format_watch_rows_json(Duration::ZERO, SystemTime::UNIX_EPOCH, &rows);
        assert_eq!(s.lines().count(), 2);
    }

    #[test]
    fn format_watch_rows_json_wall_clock_shared_across_rows_in_one_interval() {
        let rows = vec![
            watch_row(1, Some("a.exe"), 0, 0, 0, 0, false),
            watch_row(2, Some("b.exe"), 0, 0, 0, 0, false),
        ];
        let wall_clock = SystemTime::UNIX_EPOCH + Duration::from_millis(1_726_308_723_482);
        let s = format_watch_rows_json(Duration::ZERO, wall_clock, &rows);
        assert_eq!(
            s.matches(r#""wall_clock":"2024-09-14T10:12:03.482Z""#)
                .count(),
            2
        );
    }

    // --- process_sample ---

    #[cfg(feature = "test-helpers")]
    fn entry(
        pid: u32,
        name: Option<&str>,
        used_bytes: u64,
        shared_used_bytes: u64,
    ) -> GpuProcessEntry {
        GpuProcessEntry::builder()
            .pid(pid)
            .name(name.map(str::to_owned))
            .used_bytes(used_bytes)
            .shared_used_bytes(shared_used_bytes)
            .build()
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_first_interval_zero_delta() {
        let mut state = WatchState::new();
        let rows = vec![entry(100, Some("python.exe"), 8_000, 100)];
        let out = process_sample(&rows, &mut state, &[100], Duration::ZERO, None);
        assert_eq!(out.len(), 1);
        let row = out.first().unwrap();
        assert_eq!(row.used_bytes, 8_000);
        assert_eq!(row.used_delta, 0);
        assert_eq!(row.shared_delta, 0);
        // tracker: None (construction failed/not passed) means "can't
        // tell", not "measured, not spilling".
        assert_eq!(row.spilling, None);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_second_interval_computes_delta() {
        let mut state = WatchState::new();
        let rows0 = vec![entry(100, Some("python.exe"), 8_000, 100)];
        let _ = process_sample(&rows0, &mut state, &[100], Duration::ZERO, None);
        let rows1 = vec![entry(100, Some("python.exe"), 9_500, 300)];
        let out = process_sample(&rows1, &mut state, &[100], Duration::from_secs(5), None);
        let row = out.first().unwrap();
        assert_eq!(row.used_delta, 1_500);
        assert_eq!(row.shared_delta, 200);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_missing_pid_renders_zero() {
        let mut state = WatchState::new();
        let rows: Vec<GpuProcessEntry> = vec![];
        let out = process_sample(&rows, &mut state, &[42], Duration::ZERO, None);
        assert_eq!(out.len(), 1);
        let row = out.first().unwrap();
        assert_eq!(row.pid, 42);
        assert_eq!(row.used_bytes, 0);
        assert_eq!(row.name, None);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_peak_tracked_across_samples() {
        let mut state = WatchState::new();
        let rows0 = vec![entry(100, Some("python.exe"), 8_000, 100)];
        let _ = process_sample(&rows0, &mut state, &[100], Duration::ZERO, None);
        let rows1 = vec![entry(100, Some("python.exe"), 5_000, 50)];
        let _ = process_sample(&rows1, &mut state, &[100], Duration::from_secs(5), None);
        let s = state.by_pid.get(&100).unwrap();
        assert_eq!(s.peak_used_bytes, 8_000);
        assert_eq!(s.peak_shared_bytes, 100);
        assert_eq!(s.baseline_used_bytes, 8_000);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_name_change_resets_baseline_and_peak() {
        // Simulates the OS recycling pid=100 mid-watch: python.exe ran
        // for a while (baseline/peak grow), then exits and the PID is
        // reassigned to an unrelated notepad.exe. The resolved-name
        // change must reset the row's baseline/peak to the new
        // process's reading rather than mixing the two.
        let mut state = WatchState::new();
        let rows0 = vec![entry(100, Some("python.exe"), 8_000, 100)];
        let _ = process_sample(&rows0, &mut state, &[100], Duration::ZERO, None);
        let rows1 = vec![entry(100, Some("python.exe"), 9_000, 500)];
        let _ = process_sample(&rows1, &mut state, &[100], Duration::from_secs(5), None);

        let rows2 = vec![entry(100, Some("notepad.exe"), 200, 10)];
        let out = process_sample(&rows2, &mut state, &[100], Duration::from_secs(10), None);
        let row = out.first().unwrap();
        assert_eq!(row.name.as_deref(), Some("notepad.exe"));
        // Delta is 0 on the reset sample — comparing against the
        // recycled PID's own (much larger) prior reading would be
        // meaningless.
        assert_eq!(row.used_delta, 0);
        assert_eq!(row.shared_delta, 0);

        let s = state.by_pid.get(&100).unwrap();
        assert_eq!(s.baseline_used_bytes, 200);
        assert_eq!(s.baseline_shared_bytes, 10);
        assert_eq!(s.peak_used_bytes, 200);
        assert_eq!(s.peak_shared_bytes, 10);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_unresolved_name_does_not_trigger_reset() {
        // A `?` sample between two resolved samples of the SAME name
        // must not be treated as a reuse — last_name stays sticky
        // across the None sample, so the baseline is undisturbed.
        let mut state = WatchState::new();
        let rows0 = vec![entry(100, Some("python.exe"), 8_000, 100)];
        let _ = process_sample(&rows0, &mut state, &[100], Duration::ZERO, None);
        let rows1 = vec![entry(100, None, 8_500, 150)];
        let _ = process_sample(&rows1, &mut state, &[100], Duration::from_secs(5), None);
        let rows2 = vec![entry(100, Some("python.exe"), 9_000, 200)];
        let out = process_sample(&rows2, &mut state, &[100], Duration::from_secs(10), None);
        let row = out.first().unwrap();
        // Baseline never reset: delta is against the unresolved
        // sample's prev (8_500 / 150), not a fresh 0.
        assert_eq!(row.used_delta, 500);
        assert_eq!(row.shared_delta, 50);
        assert_eq!(state.by_pid.get(&100).unwrap().baseline_used_bytes, 8_000);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_protected_bracket_flicker_does_not_trigger_reset() {
        // Same shape as the `None`-flicker test above, but for the
        // Windows-only `[protected]` synthetic bracket: a transient
        // Toolhelp32Snapshot failure on one interval must not look like
        // "the OS recycled this PID" and reset the baseline.
        let mut state = WatchState::new();
        let rows0 = vec![entry(100, Some("python.exe"), 8_000, 100)];
        let _ = process_sample(&rows0, &mut state, &[100], Duration::ZERO, None);
        let rows1 = vec![entry(100, Some("[protected]"), 8_500, 150)];
        let _ = process_sample(&rows1, &mut state, &[100], Duration::from_secs(5), None);
        let rows2 = vec![entry(100, Some("python.exe"), 9_000, 200)];
        let out = process_sample(&rows2, &mut state, &[100], Duration::from_secs(10), None);
        let row = out.first().unwrap();
        assert_eq!(row.used_delta, 500);
        assert_eq!(row.shared_delta, 50);
        assert_eq!(state.by_pid.get(&100).unwrap().baseline_used_bytes, 8_000);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_reset_survives_a_protected_flicker_in_between() {
        // A `[protected]` flicker must not mask a GENUINE later name
        // change either: last_name should stay "python.exe" (sticky
        // across the flicker), so the real reuse at rows2 still resets.
        let mut state = WatchState::new();
        let rows0 = vec![entry(100, Some("python.exe"), 8_000, 100)];
        let _ = process_sample(&rows0, &mut state, &[100], Duration::ZERO, None);
        let rows1 = vec![entry(100, Some("[protected]"), 8_500, 150)];
        let _ = process_sample(&rows1, &mut state, &[100], Duration::from_secs(5), None);
        let rows2 = vec![entry(100, Some("notepad.exe"), 200, 10)];
        let out = process_sample(&rows2, &mut state, &[100], Duration::from_secs(10), None);
        let row = out.first().unwrap();
        assert_eq!(row.used_delta, 0);
        assert_eq!(row.shared_delta, 0);
        let s = state.by_pid.get(&100).unwrap();
        assert_eq!(s.baseline_used_bytes, 200);
        assert_eq!(s.baseline_shared_bytes, 10);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_growth_hint_fires_for_protected_bracket() {
        // The one-shot "unresolved pid grew" stderr hint must still
        // fire for the Windows-only `[protected]` bracket, not just a
        // bare `None` — otherwise the hint would go silent on Windows
        // now that most `?` rows resolve via the Toolhelp32Snapshot
        // fallback and only genuinely-protected rows stay unresolved.
        let mut state = WatchState::new();
        let rows0 = vec![entry(100, Some("[protected]"), 0, 0)];
        let _ = process_sample(&rows0, &mut state, &[100], Duration::ZERO, None);
        let grown = UNRESOLVED_GROWTH_HINT_BYTES;
        let rows1 = vec![entry(100, Some("[protected]"), grown, 0)];
        let _ = process_sample(&rows1, &mut state, &[100], Duration::from_secs(5), None);
        assert!(state.by_pid.get(&100).unwrap().growth_hint_fired);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn process_sample_growth_hint_does_not_fire_for_exited_bracket() {
        // `[exited]` means the process was already confirmed gone —
        // "growth" on a gone process is meaningless, so the hint (whose
        // wording promises "re-run elevated to identify") must not fire
        // for it the way it does for `None`/`[protected]`.
        let mut state = WatchState::new();
        let rows0 = vec![entry(100, Some("[exited]"), 0, 0)];
        let _ = process_sample(&rows0, &mut state, &[100], Duration::ZERO, None);
        let grown = UNRESOLVED_GROWTH_HINT_BYTES;
        let rows1 = vec![entry(100, Some("[exited]"), grown, 0)];
        let _ = process_sample(&rows1, &mut state, &[100], Duration::from_secs(5), None);
        assert!(!state.by_pid.get(&100).unwrap().growth_hint_fired);
    }

    // --- resolved_name (PID-reuse comparison filter) ---

    #[test]
    fn resolved_name_passes_through_real_names() {
        assert_eq!(resolved_name(Some("python.exe")), Some("python.exe"));
    }

    #[test]
    fn resolved_name_passes_through_kernel_bracket() {
        // [kernel] (PID 4) is permanently stable and never flickers —
        // safe to treat as a comparable name, unlike [protected]/[exited].
        assert_eq!(resolved_name(Some("[kernel]")), Some("[kernel]"));
    }

    #[test]
    fn resolved_name_filters_protected_and_exited_brackets() {
        assert_eq!(resolved_name(Some("[protected]")), None);
        assert_eq!(resolved_name(Some("[exited]")), None);
    }

    #[test]
    fn resolved_name_filters_none() {
        assert_eq!(resolved_name(None), None);
    }

    // --- WatchState::track (--follow-new seen_order bookkeeping) ---

    #[test]
    fn watch_state_track_records_first_seen_order_once() {
        let mut state = WatchState::new();
        state.track(100, 1_000, 0);
        state.track(200, 2_000, 0);
        // Re-tracking an already-seen PID must not append a second
        // seen_order entry, nor reset its state.
        state.track(100, 9_000, 0);
        assert_eq!(state.seen_order, vec![100, 200]);
        assert_eq!(state.by_pid.get(&100).unwrap().baseline_used_bytes, 1_000);
    }

    #[test]
    fn watch_state_track_returns_mutable_existing_entry() {
        let mut state = WatchState::new();
        state.track(100, 1_000, 0).peak_used_bytes = 5_000;
        // The second `track` call for the same PID must see the
        // mutation the first call's caller made through the returned
        // reference, not a fresh `WatchedPidState`.
        assert_eq!(state.track(100, 1_000, 0).peak_used_bytes, 5_000);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn watch_state_seen_order_survives_pid_dropping_out_of_watched() {
        // Simulates --follow-new: pid 100 is watched for one interval,
        // genuinely excluded from `watched` the next (dropped below
        // top-N, but still alive and still present in `rows` — only
        // absent from the `watched` slice `process_sample` is given),
        // then re-enters. seen_order must record it exactly once, at
        // its first sighting, and its accumulated state must survive
        // the gap untouched rather than drifting or resetting.
        let mut state = WatchState::new();
        let rows0 = vec![
            entry(100, Some("a.exe"), 1_000, 0),
            entry(200, Some("b.exe"), 500, 0),
        ];
        let _ = process_sample(&rows0, &mut state, &[100, 200], Duration::ZERO, None);

        // pid 100 is genuinely excluded from `watched` this interval
        // even though it's still present in `rows` at a different
        // reading (9_000) — process_sample must not touch its state
        // at all while it's excluded.
        let rows1 = vec![
            entry(100, Some("a.exe"), 9_000, 0),
            entry(200, Some("b.exe"), 600, 0),
        ];
        let _ = process_sample(&rows1, &mut state, &[200], Duration::from_secs(5), None);
        assert_eq!(
            state.by_pid.get(&100).unwrap().peak_used_bytes,
            1_000,
            "excluded PID's state must be untouched while absent from `watched`"
        );

        // pid 100 re-enters `watched`; its delta is against its own
        // pre-gap prev (1_000), not the 9_000 it drifted to while
        // excluded — process_sample never saw that reading.
        let rows2 = vec![
            entry(100, Some("a.exe"), 1_200, 0),
            entry(200, Some("b.exe"), 600, 0),
        ];
        let out = process_sample(
            &rows2,
            &mut state,
            &[100, 200],
            Duration::from_secs(10),
            None,
        );
        let row100 = out.iter().find(|r| r.pid == 100).unwrap();
        assert_eq!(row100.used_delta, 200); // 1_200 - 1_000, not 1_200 - 9_000
        assert_eq!(state.by_pid.get(&100).unwrap().peak_used_bytes, 1_200);
        assert_eq!(state.seen_order, vec![100, 200]);
    }

    // --- Selection ---

    /// An auto-selection `Selection` with the given `--filter` patterns.
    fn auto(top: usize, follow_new: bool, filters: &[&str]) -> Selection {
        let filters: Vec<String> = filters.iter().map(|f| (*f).to_owned()).collect();
        Selection::new(&[], top, follow_new, &filters).unwrap()
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn selection_explicit_passthrough_ignores_rows_and_top() {
        let rows = vec![entry(1, Some("a.exe"), 9_000, 0)];
        let sel = Selection::new(&[42, 43], 1, false, &[]).unwrap();
        assert_eq!(sel.select(&rows, &WatchState::new()).pids, vec![42, 43]);
    }

    #[test]
    fn selection_explicit_pids_are_deduplicated_in_order() {
        let sel = Selection::new(&[7, 3, 7, 3, 9], 5, false, &[]).unwrap();
        assert_eq!(sel.select(&[], &WatchState::new()).pids, vec![7, 3, 9]);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn selection_auto_selects_top_n_from_rows() {
        let rows = vec![
            entry(1, Some("a.exe"), 1_000, 0),
            entry(2, Some("b.exe"), 5_000, 0),
            entry(3, Some("c.exe"), 3_000, 0),
        ];
        assert_eq!(
            auto(2, false, &[]).select(&rows, &WatchState::new()).pids,
            vec![2, 3]
        );
    }

    #[test]
    fn selection_empty_rows_and_explicit_is_empty() {
        let selected = auto(5, false, &[]).select(&[], &WatchState::new());
        assert!(selected.pids.is_empty());
        assert!(selected.unmatchable.is_empty());
    }

    #[test]
    fn selection_rejects_follow_new_with_explicit_pids() {
        let err = Selection::new(&[42], 5, true, &[]).err().unwrap();
        // Byte-identical to the pre-v0.2.12 message (`main` adds `hmn: `).
        assert_eq!(
            err,
            "watch --follow-new only applies to auto-selection; drop --follow-new or the \
             explicit PID list"
        );
    }

    #[test]
    fn selection_rejects_filter_with_explicit_pids() {
        let err = Selection::new(&[42], 5, false, &["train".to_owned()])
            .err()
            .unwrap();
        assert_eq!(
            err,
            "watch --filter only applies to auto-selection; drop --filter or the explicit PID \
             list"
        );
    }

    #[test]
    fn selection_describe_matches_the_pre_selection_header_strings() {
        // Without `--filter`, the three header clauses `run_watch` printed
        // before `Selection` existed, byte for byte.
        assert_eq!(
            auto(3, true, &[]).describe(1),
            "following top 3 by committed (re-selected every interval), 1 initially"
        );
        assert_eq!(
            auto(5, false, &[]).describe(4),
            "watching 4 PID(s) (top 5 by committed)"
        );
        assert_eq!(
            Selection::new(&[10, 11], 5, false, &[])
                .unwrap()
                .describe(2),
            "watching 2 PID(s)"
        );
    }

    #[test]
    fn selection_describe_announces_the_filter() {
        assert_eq!(
            auto(3, true, &["figure13"]).describe(1),
            "following top 3 by committed among names containing \"figure13\" \
             (case-insensitive) (re-selected every interval), 1 initially"
        );
        assert_eq!(
            auto(5, false, &["train", "eval run"]).describe(2),
            "watching 2 PID(s) (top 5 by committed among names containing \"train\" or \
             \"eval run\" (case-insensitive))"
        );
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn selection_filter_narrows_before_top_n() {
        // `dwm.exe` holds the most VRAM, so rank alone would pick it; the
        // filter admits only the two `train*` processes, then top-1 picks
        // the larger of them.
        let rows = vec![
            entry(1, Some("dwm.exe"), 9_000, 0),
            entry(2, Some("train.exe"), 3_000, 0),
            entry(3, Some("Train_Eval.EXE"), 5_000, 0),
        ];
        let state = WatchState::new();
        assert_eq!(
            auto(1, true, &["TRAIN"]).select(&rows, &state).pids,
            vec![3]
        );
        assert_eq!(
            auto(5, true, &["train"]).select(&rows, &state).pids,
            vec![3, 2]
        );
        assert!(
            auto(5, true, &["nope"])
                .select(&rows, &state)
                .pids
                .is_empty()
        );
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn selection_filter_patterns_are_or_ed() {
        let rows = vec![
            entry(1, Some("python.exe"), 1_000, 0),
            entry(2, Some("train.exe"), 2_000, 0),
            entry(3, Some("dwm.exe"), 3_000, 0),
        ];
        let got = auto(5, true, &["python", "train"]).select(&rows, &WatchState::new());
        assert_eq!(got.pids, vec![2, 1]);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn selection_filter_keeps_a_followed_pid_through_a_name_flicker() {
        // pid 9 was followed as `train.exe`; this sample its name reads
        // `[protected]`. The sticky last-resolved name keeps it matching.
        let mut state = WatchState::new();
        state.track(9, 0, 0).last_name = Some("train.exe".to_owned());
        let rows = vec![entry(9, Some("[protected]"), 4_000, 0)];
        let got = auto(3, true, &["train"]).select(&rows, &state);
        assert_eq!(got.pids, vec![9]);
        assert!(got.unmatchable.is_empty());
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn selection_filter_reports_never_resolved_rows_as_unmatchable() {
        let rows = vec![
            entry(1, Some("[protected]"), 1_000, 0),
            entry(2, Some("[exited]"), 1_000, 0),
            entry(3, Some("?"), 1_000, 0),
            entry(4, None, 1_000, 0),
            entry(5, Some("train.exe"), 1_000, 0),
            entry(6, Some("dwm.exe"), 1_000, 0),
        ];
        let got = auto(9, true, &["train"]).select(&rows, &WatchState::new());
        assert_eq!(got.pids, vec![5]);
        // Only the rows the filter could not judge — `dwm.exe` was judged,
        // and rejected.
        assert_eq!(got.unmatchable, vec![1, 2, 3, 4]);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn selection_without_filter_reports_nothing_unmatchable() {
        let rows = vec![entry(1, Some("[protected]"), 1_000, 0)];
        let got = auto(3, true, &[]).select(&rows, &WatchState::new());
        assert_eq!(got.pids, vec![1]);
        assert!(got.unmatchable.is_empty());
    }

    #[test]
    fn matchable_name_prefers_current_then_sticky() {
        assert_eq!(matchable_name(Some("a.exe"), Some("b.exe")), Some("a.exe"));
        assert_eq!(
            matchable_name(Some("[protected]"), Some("b.exe")),
            Some("b.exe")
        );
        assert_eq!(
            matchable_name(Some("[exited]"), Some("b.exe")),
            Some("b.exe")
        );
        assert_eq!(matchable_name(Some("?"), Some("b.exe")), Some("b.exe"));
        assert_eq!(matchable_name(None, Some("b.exe")), Some("b.exe"));
        assert_eq!(matchable_name(Some("?"), Some("?")), None);
        assert_eq!(matchable_name(None, None), None);
        // `[kernel]` is a stable, genuine name (PID 4), as for PID reuse.
        assert_eq!(matchable_name(Some("[kernel]"), None), Some("[kernel]"));
    }

    #[test]
    fn matches_any_is_a_case_insensitive_substring_or() {
        let pats = ["Figure13".to_owned(), "python".to_owned()];
        assert!(matches_any("figure13_newline_patch.exe", &pats));
        assert!(matches_any("PYTHON.EXE", &pats));
        assert!(!matches_any("dwm.exe", &pats));
    }

    #[test]
    fn unmatchable_notices_announce_each_pid_once() {
        let mut announced = HashSet::new();
        assert_eq!(
            unmatchable_notices(&[7, 8], &mut announced),
            [
                "hmn watch: pid=7 has no resolvable name; --filter cannot match it",
                "hmn watch: pid=8 has no resolvable name; --filter cannot match it",
            ]
        );
        assert_eq!(
            unmatchable_notices(&[8, 9], &mut announced),
            ["hmn watch: pid=9 has no resolvable name; --filter cannot match it"]
        );
    }

    #[test]
    fn parse_filter_pattern_rejects_blank() {
        assert_eq!(parse_filter_pattern("train").unwrap(), "train");
        assert!(parse_filter_pattern("").is_err());
        assert!(parse_filter_pattern("   ").is_err());
    }

    // --- format_followed_set_change (--follow-new stderr breadcrumb) ---

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_followed_set_change_none_when_unchanged() {
        let rows = vec![entry(1, Some("a.exe"), 1_000, 0)];
        let state = WatchState::new();
        assert!(format_followed_set_change(&[1], &[1], &rows, &state, Duration::ZERO).is_none());
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_followed_set_change_reports_entered_with_name_from_rows() {
        let rows = vec![entry(2, Some("new.exe"), 1_000, 0)];
        let state = WatchState::new();
        let msg = format_followed_set_change(&[1], &[1, 2], &rows, &state, Duration::from_secs(10))
            .unwrap();
        assert!(msg.contains("entered pid=2 (new.exe)"), "{msg}");
        assert!(!msg.contains("left"), "{msg}");
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_followed_set_change_reports_left_with_name_from_state() {
        // The departed PID is, by construction, absent from the current
        // sample's `rows` — its name must come from `state`'s last
        // known reading instead.
        let rows: Vec<GpuProcessEntry> = vec![];
        let mut state = WatchState::new();
        state.track(1, 1_000, 0).last_name = Some("gone.exe".to_owned());
        let msg =
            format_followed_set_change(&[1], &[], &rows, &state, Duration::from_secs(10)).unwrap();
        assert!(msg.contains("left pid=1 (gone.exe)"), "{msg}");
        assert!(!msg.contains("entered"), "{msg}");
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_followed_set_change_unresolved_name_renders_bare_pid() {
        let rows = vec![entry(2, None, 1_000, 0)];
        let state = WatchState::new();
        let msg = format_followed_set_change(&[1], &[1, 2], &rows, &state, Duration::ZERO).unwrap();
        assert!(msg.contains("entered pid=2"), "{msg}");
        assert!(!msg.contains("pid=2 ("), "{msg}");
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_followed_set_change_reports_both_entered_and_left() {
        let rows = vec![entry(2, Some("new.exe"), 1_000, 0)];
        let state = WatchState::new();
        let msg = format_followed_set_change(&[1], &[2], &rows, &state, Duration::ZERO).unwrap();
        assert!(msg.contains("entered pid=2 (new.exe)"), "{msg}");
        assert!(msg.contains("left pid=1"), "{msg}");
    }

    // --- format_watch_per_pid_block / summary formatting ---

    #[test]
    fn format_watch_per_pid_block_empty_is_empty_string() {
        assert_eq!(format_watch_per_pid_block(&[]), "");
    }

    #[test]
    fn format_watch_per_pid_block_single_pid() {
        let s = format_watch_per_pid_block(&[pid_summary(
            12345,
            Some("python.exe"),
            8 * 1024 * 1024 * 1024,
            9 * 1024 * 1024 * 1024,
            100 * 1024 * 1024,
            700 * 1024 * 1024,
        )]);
        assert!(s.contains("12345"));
        assert!(s.contains("python.exe"));
        assert!(s.contains("8.0 GiB"));
        assert!(s.contains("9.0 GiB"));
        assert!(s.contains("100 MiB"));
        assert!(s.contains("700 MiB"));
    }

    #[test]
    fn format_watch_summary_text_unmeasurable_notes_and_still_shows_per_pid() {
        let s = format_watch_summary_text(None, &[pid_summary(1, Some("a.exe"), 0, 0, 0, 0)]);
        assert!(s.contains("spill tracking unavailable"));
        assert!(s.contains("per-PID"));
        assert!(s.contains('1'));
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_watch_summary_text_measurable_uses_watch_prefix() {
        let s = format_watch_summary_text(Some(&spilling_report()), &[]);
        assert!(s.starts_with("hmn watch: peak dedicated"));
    }

    #[test]
    fn format_watch_summary_json_unmeasurable_shape() {
        let s = format_watch_summary_json(None, &[pid_summary(1, Some("a.exe"), 10, 20, 0, 0)]);
        assert!(s.starts_with(
            r#"{"kind":"summary","measurable":false,"spilled":false,"observations":0,"#
        ));
        assert!(s.contains(r#""per_pid":[{"pid":1,"name":"a.exe","baseline_used_bytes":10,"peak_used_bytes":20,"baseline_shared_bytes":0,"peak_shared_bytes":0}]"#));
        assert!(s.ends_with("]}\n"));
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_watch_summary_json_measurable_shape() {
        let s = format_watch_summary_json(Some(&spilling_report()), &[]);
        assert!(s.starts_with(r#"{"kind":"summary","measurable":true,"spilled":true,"#));
        assert!(s.contains(r#""per_pid":[]"#));
    }
}
