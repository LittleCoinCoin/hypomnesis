// SPDX-License-Identifier: MIT OR Apache-2.0

//! `hmn ps`: the GPU-process listing — its row model, sort keys and
//! comparator, the stderr summary line, and its text and JSON renderers.

use std::fmt::Write as _;

use clap::ValueEnum;
use hypomnesis::{Result, device_count, device_info, gpu_processes, snapshot_is_spilling};

use crate::format::{column_width, format_vram, format_vram_precise, json_escape, spill_cell};

/// One row of `hmn ps` output (binary-internal — not part of the
/// library's public API).
#[derive(Debug, Clone)]
pub struct PsRow {
    /// Process ID.
    pub pid: u32,
    /// Process name. `None` when no name source produced one.
    pub name: Option<String>,
    /// GPU memory used by this process in bytes (`WDDM` dedicated
    /// commit on the Windows `PDH` path).
    pub used_bytes: u64,
    /// Resident shared-system-memory bytes — the `WDDM` spill signal.
    /// `0` on non-Windows backends (no shared-residency counter).
    pub shared_used_bytes: u64,
    /// Zero-based device index (NVML-canonical).
    pub device_index: u32,
    /// Friendly device name (e.g. `RTX 5060 Ti`); `None` when
    /// `device_info` failed for this index.
    pub device_name: Option<String>,
    /// One-shot spill check ([`hypomnesis::snapshot_is_spilling`]) for
    /// this row's device, computed once per device and broadcast to
    /// every row on it — same "adapter-wide, same value on every row"
    /// shape `hmn watch`'s `spilling` field already uses. `None` when
    /// not measurable (non-Windows, pre-`WDDM 2.0`, non-NVIDIA adapter,
    /// or a live `PDH` sample failure) — never collapsed into
    /// `Some(false)`.
    pub spilling: Option<bool>,
}

/// Display-order key for `hmn ps --sort` (and, always pinned to
/// [`Self::Dedicated`], for `watch::select_top_n_pids`'s auto-selection).
///
/// Binary-internal dispatch enum, not a library type — matched
/// exhaustively by [`ps_row_comparator`], the sole place that
/// interprets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortKey {
    /// `used_bytes` (`WDDM` dedicated commit) descending — "who do I
    /// kill to free VRAM?". The default; matches `hmn ps`'s pre-v0.2.7
    /// fixed order exactly. Also accepts `vram` (the column header and
    /// the word the rest of the tool's help text uses for this
    /// quantity) and `committed` (the word `hmn watch`'s `COMMITTED`
    /// column uses for the same quantity) as aliases — same ordering,
    /// different vocabulary entry points, so users don't have to learn
    /// `hmn`-internal naming to reach for the default sort.
    #[value(alias = "vram", alias = "committed")]
    Dedicated,
    /// `shared_used_bytes` (resident shared-system-memory, the spill
    /// signal) descending — "who is currently being paged out?". A
    /// symptom, not a cause: a process high in SHARED has already lost
    /// the fight for dedicated VRAM. Always a no-op ordering on Linux
    /// and macOS, where `shared_used_bytes` is always `0`.
    Shared,
    /// `used_bytes + shared_used_bytes` descending — "who is the
    /// biggest GPU-memory citizen overall?". Outweighs `Dedicated` for
    /// processes that hold meaningful shared residency alongside their
    /// dedicated commit.
    Total,
}

/// Build the row comparator for a given [`SortKey`], shared by `hmn ps`
/// (user-selectable via `--sort`) and `watch::select_top_n_pids` (always
/// [`SortKey::Dedicated`]) so the two orderings cannot silently drift
/// apart. Tie-breaks (name ascending, then PID ascending — stable
/// output across runs, and clusters duplicate-name processes like
/// `msedgewebview2.exe`) are identical regardless of the primary key.
pub const fn ps_row_comparator(key: SortKey) -> impl Fn(&PsRow, &PsRow) -> std::cmp::Ordering {
    move |a, b| {
        let primary = match key {
            SortKey::Dedicated => b.used_bytes.cmp(&a.used_bytes),
            SortKey::Shared => b.shared_used_bytes.cmp(&a.shared_used_bytes),
            SortKey::Total => b
                .used_bytes
                .saturating_add(b.shared_used_bytes)
                .cmp(&a.used_bytes.saturating_add(a.shared_used_bytes)),
        };
        primary
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.pid.cmp(&b.pid))
    }
}

/// Run the `ps` subcommand: collect process rows for the selected
/// device(s) — sampling one live adapter-wide spill check per device
/// along the way (see [`snapshot_is_spilling`]) — apply the `--pid` /
/// `--min` filters, sort per `--sort`, then emit either a text table
/// or JSON.
//
// Returns `Result<()>` for symmetry with `run_summary` so `main` can
// dispatch through one match arm. The body never produces an `Err` (per-device
// failures are swallowed via `continue` so one broken device doesn't kill the
// whole listing); the lint is allowed for that reason.
#[allow(clippy::unnecessary_wraps)]
pub fn run_ps(
    pid_filter: Option<u32>,
    device_filter: Option<u32>,
    min_filter: Option<u64>,
    sort: SortKey,
    json: bool,
) -> Result<()> {
    // device_count returning Err here means no enumeration backend is
    // enabled / every backend failed; treat as zero NVIDIA devices and
    // let the empty Vec fall through to the formatter (which prints
    // a header-only table or `[]`).
    let device_indices: Vec<u32> = device_filter.map_or_else(
        || (0..device_count().unwrap_or(0)).collect(),
        |idx| vec![idx],
    );

    let mut rows: Vec<PsRow> = Vec::new();
    for &idx in &device_indices {
        // Look up the device name once per device for the DEVICE column.
        // Failure here is non-fatal: row's `device_name` falls back to
        // None and the formatter renders `GPU N` instead.
        let device_name = device_info(idx).ok().and_then(|d| d.name);
        // One live spill sample per device (not per row): `snapshot_is_spilling`
        // is adapter-wide, so every row on this device gets the same
        // value — the same "broadcast" shape `hmn watch`'s `spilling`
        // field already uses. `None` (not measurable) on non-Windows,
        // pre-WDDM-2.0, a non-NVIDIA adapter, or a PDH hiccup.
        //
        // Sampled *before* gpu_processes(idx), not after: the SHARED
        // column on each row and the SPILL verdict broadcast onto it
        // should describe the same instant. Sampling after would let a
        // process's per-process PDH enumeration (which gpu_processes
        // performs) and the Toolhelp32Snapshot name-resolution walk
        // elapse in between — real time under load — so a job that
        // starts or stops spilling in that gap would show a SHARED
        // figure and a SPILL verdict from two different moments. This
        // does mean the PDH open+sample below still runs even for a
        // device every row of which the --pid/--min filters end up
        // dropping; that's the accepted trade (measured negligible on
        // the reference machine — see CHANGELOG) for not straddling
        // gpu_processes()'s own call duration, the same call-ordering
        // discipline `hmn watch`'s wall_clock/t_ms pairing uses.
        let spilling = snapshot_is_spilling(idx);
        let Ok(entries) = gpu_processes(idx) else {
            continue;
        };
        for entry in entries {
            if let Some(want) = pid_filter
                && entry.pid != want
            {
                continue;
            }
            if let Some(min) = min_filter
                && entry.used_bytes.saturating_add(entry.shared_used_bytes) < min
            {
                continue;
            }
            rows.push(PsRow {
                pid: entry.pid,
                name: entry.name,
                used_bytes: entry.used_bytes,
                shared_used_bytes: entry.shared_used_bytes,
                device_index: idx,
                // BORROW: clone — device_name is shared across all
                // rows for this device.
                device_name: device_name.clone(),
                spilling,
            });
        }
    }

    // Human-facing display order, per `--sort` (default: VRAM descending
    // so the biggest consumers land at the top — the row a user asking
    // "what's eating my GPU memory?" wants to see first). Tie-breaks
    // (name ascending for grouping duplicate-name processes like
    // `msedgewebview2.exe`, then PID ascending for stable order across
    // runs) are identical regardless of key — see `ps_row_comparator`.
    // The library's `gpu_processes()` returns rows PID-sorted; this
    // overrides that for display only.
    rows.sort_by(ps_row_comparator(sort));

    if json {
        print!("{}", format_ps_json(&rows));
    } else {
        print!("{}", format_ps_table(&rows));
    }
    // Human-readable summary on stderr — preserves stdout's scriptability
    // (header-only table or `[]` for empty) while giving interactive
    // users an unambiguous "command worked, here's the count" line.
    // Always printed, even when rows is non-empty, so the message is a
    // consistent confirmation rather than an error indicator. Redirect
    // 2>/dev/null to suppress.
    eprintln!(
        "hmn: {}",
        format_ps_summary(&rows, pid_filter, device_filter, min_filter)
    );
    Ok(())
}

/// Build the stderr summary string for `hmn ps`. Format:
/// `<N> GPU process[es] found[ matching <filters>][ (<X.Y> <unit> committed total[; <M> protected — re-run elevated for names)].`
///
/// Two appendices after the noun, each elided when not applicable:
///
/// - **Filter clause** (` matching pid=N device=M min=X unit`):
///   appended only when at least one filter is active. Supports any
///   combination of `--pid`, `--device`, and `--min` (echoed via
///   [`format_vram_precise`], not [`format_vram`], so a sub-MiB or
///   otherwise-imprecise `--min` value is never misreported).
/// - **Committed-total parenthetical** (` (X.Y unit committed total)`,
///   formatted via [`format_vram`] so it renders as `MiB` below 1
///   `GiB` and `GiB` to one decimal place otherwise): appended only
///   when `count > 0`. The word "committed" hints at the `WDDM`
///   commit-vs-resident distinction the Windows `PDH` backend
///   exposes — summing `used_bytes` across processes can exceed
///   physical `VRAM` under `WDDM` (a real `WDDM` property, not a
///   bug), so naming the figure "committed total" prevents that from
///   reading as broken when a Windows user sees, say, 32 `GiB`
///   committed on a 16 `GiB` card. Elided entirely when `count == 0`
///   because a zero-bytes total carries no information.
///
///   When at least one row is genuinely unresolvable, the parenthetical
///   carries a **protected continuation**
///   (`; M protected — re-run elevated for names`) joined by `; `. A row
///   counts as protected when `name.is_none()` (`NVML`'s
///   `/proc/<pid>/comm` unreadable on Linux; macOS cross-user PIDs whose
///   `ledger` syscall returned `EPERM` — `sudo hmn ps` is the equivalent
///   elevation there); when `name` is exactly `Some("[protected]")` (the
///   Windows-only bracket meaning the `Toolhelp32Snapshot` fallback could
///   not be taken at all — see `hypomnesis::gpu_processes`'s Windows
///   path); or when `name` is the literal `Some("?")` string the
///   pre-`WDDM 2.0` `nvidia-smi` fallback writes for a row it couldn't
///   name itself (same "might resolve under elevation" meaning as the
///   other two — pre-existing, but not previously counted here). `PID 4`
///   (`[kernel]`) and `[exited]` rows deliberately
///   do **not** contribute to this count: `[kernel]` has no executable
///   image to resolve regardless of privilege, and `[exited]` means the
///   process was already gone by the time of the name lookup — elevation
///   would not have helped either case, so counting them would overstate
///   what re-running elevated could actually buy. As of v0.2.8, most
///   Windows `?` rows resolve to a real name via the snapshot fallback
///   before this function ever sees them; the count that remains is
///   genuinely foreign-user / `SYSTEM` / `PPL`-protected processes, or —
///   on Linux/macOS, where the fallback doesn't apply — any unresolved
///   row at all.
///
/// "GPU process" / "GPU processes" (not the previous-release
/// "compute process" / "compute processes") because on the `PDH`
/// Windows path the list includes every GPU memory holder
/// (compositor, browsers, games, compute), not just `CUDA` contexts.
fn format_ps_summary(
    rows: &[PsRow],
    pid_filter: Option<u32>,
    device_filter: Option<u32>,
    min_filter: Option<u64>,
) -> String {
    let count = rows.len();
    let protected = rows
        .iter()
        .filter(|r| {
            r.name.is_none()
                || r.name.as_deref() == Some("[protected]")
                || r.name.as_deref() == Some("?")
        })
        .count();
    let committed_total: u64 = rows.iter().map(|r| r.used_bytes).sum();

    let noun = if count == 1 {
        "GPU process"
    } else {
        "GPU processes"
    };

    let mut out = format!("{count} {noun} found");

    // Vec rather than a fixed-arity match: three independent optional
    // filters compose more clearly as "push what's present, join with
    // spaces" than as an 8-arm match on a 3-tuple.
    let mut clauses: Vec<String> = Vec::new();
    if let Some(p) = pid_filter {
        clauses.push(format!("pid={p}"));
    }
    if let Some(d) = device_filter {
        clauses.push(format!("device={d}"));
    }
    if let Some(m) = min_filter {
        // format_vram_precise, not format_vram: pid=/device= echo exact
        // values, and format_vram's MiB-below-1-GiB rounding would
        // print a real sub-MiB --min as "0 MiB" — indistinguishable
        // from the documented --min 0 no-op.
        clauses.push(format!("min={}", format_vram_precise(m)));
    }
    if !clauses.is_empty() {
        let _ = write!(out, " matching {}", clauses.join(" "));
    }

    // Committed-total + protected parenthetical. The word "committed"
    // hints at the WDDM commit-vs-resident distinction the Windows
    // backend exposes — summing `used_bytes` across processes can
    // exceed physical VRAM under WDDM (a real WDDM property, not a
    // bug), so naming the figure "committed total" prevents that from
    // reading as broken. Elided entirely when `count == 0` because
    // "0 MiB committed total" carries no information.
    match (count, protected) {
        (0, _) => {}
        (_, 0) => {
            let _ = write!(out, " ({} committed total)", format_vram(committed_total));
        }
        (_, p) => {
            let _ = write!(
                out,
                " ({} committed total; {p} protected — re-run elevated for names)",
                format_vram(committed_total)
            );
        }
    }

    out.push('.');
    out
}

/// Format `ps` rows as a fixed-column text table. Always prints the
/// header, even when `rows` is empty.
#[allow(clippy::missing_panics_doc)] // writes to a String; cannot fail in practice
fn format_ps_table(rows: &[PsRow]) -> String {
    let pid_header = "PID";
    let name_header = "NAME";
    let vram_header = "VRAM";
    let shared_header = "SHARED";
    let device_header = "DEVICE";
    let spill_header = "SPILL";

    let pid_cells: Vec<String> = rows.iter().map(|r| r.pid.to_string()).collect();
    let name_cells: Vec<&str> = rows
        .iter()
        .map(|r| r.name.as_deref().unwrap_or("?"))
        .collect();
    let vram_cells: Vec<String> = rows.iter().map(|r| format_vram(r.used_bytes)).collect();
    let shared_cells: Vec<String> = rows
        .iter()
        .map(|r| format_vram(r.shared_used_bytes))
        .collect();
    let device_cells: Vec<String> = rows
        .iter()
        .map(|r| {
            r.device_name
                .clone()
                .unwrap_or_else(|| format!("GPU {}", r.device_index))
        })
        .collect();
    let spill_cells: Vec<&str> = rows.iter().map(|r| spill_cell(r.spilling)).collect();

    let pid_w = column_width(pid_header, pid_cells.iter().map(String::as_str));
    let name_w = column_width(name_header, name_cells.iter().copied());
    let vram_w = column_width(vram_header, vram_cells.iter().map(String::as_str));
    let shared_w = column_width(shared_header, shared_cells.iter().map(String::as_str));
    let device_w = column_width(device_header, device_cells.iter().map(String::as_str));
    let spill_w = column_width(spill_header, spill_cells.iter().copied());

    let mut out = String::new();
    let _ = writeln!(
        out,
        "{pid_header:<pid_w$}  {name_header:<name_w$}  {vram_header:<vram_w$}  {shared_header:<shared_w$}  {device_header:<device_w$}  {spill_header:<spill_w$}",
    );
    for (((((pid, name), vram), shared), device), spill) in pid_cells
        .iter()
        .zip(&name_cells)
        .zip(&vram_cells)
        .zip(&shared_cells)
        .zip(&device_cells)
        .zip(&spill_cells)
    {
        let _ = writeln!(
            out,
            "{pid:<pid_w$}  {name:<name_w$}  {vram:<vram_w$}  {shared:<shared_w$}  {device:<device_w$}  {spill:<spill_w$}",
        );
    }
    out
}

/// Format `ps` rows as a JSON array, one object per row. Hand-rolled
/// (no `serde` dep — keeps the `cli` feature lean for v0.2). Each
/// object: `{"pid":N,"name":<string|null>,"used_bytes":N,"shared_used_bytes":N,"device_index":N,"device_name":<string|null>,"spilling":<true|false|null>}`.
/// `spilling` is `null`, never `false`, when spill isn't measurable
/// here — see [`PsRow::spilling`]'s doc. String values are
/// JSON-escaped via [`json_escape`].
#[allow(clippy::missing_panics_doc)] // writes to a String; cannot fail in practice
fn format_ps_json(rows: &[PsRow]) -> String {
    let mut out = String::from("[");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let name_json = row.name.as_deref().map_or_else(
            || String::from("null"),
            |n| format!("\"{}\"", json_escape(n)),
        );
        let device_name_json = row.device_name.as_deref().map_or_else(
            || String::from("null"),
            |n| format!("\"{}\"", json_escape(n)),
        );
        let spilling_json = match row.spilling {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        };
        let _ = write!(
            out,
            r#"{{"pid":{},"name":{name_json},"used_bytes":{},"shared_used_bytes":{},"device_index":{},"device_name":{device_name_json},"spilling":{spilling_json}}}"#,
            row.pid, row.used_bytes, row.shared_used_bytes, row.device_index,
        );
    }
    out.push_str("]\n");
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_docs_in_private_items
)]
mod tests {
    use super::*;
    use crate::test_support::row;

    /// Like [`row`] but with a non-zero `shared_used_bytes` — for the
    /// SHARED-column / spill-signal specific tests.
    fn row_shared(pid: u32, name: Option<&str>, used_bytes: u64, shared_used_bytes: u64) -> PsRow {
        PsRow {
            pid,
            name: name.map(str::to_owned),
            used_bytes,
            shared_used_bytes,
            device_index: 0,
            device_name: None,
            spilling: None,
        }
    }

    /// Like [`row`] but with an explicit `spilling` — for the SPILL
    /// column / field specific tests.
    fn row_spilling(pid: u32, name: Option<&str>, spilling: Option<bool>) -> PsRow {
        PsRow {
            pid,
            name: name.map(str::to_owned),
            used_bytes: 0,
            shared_used_bytes: 0,
            device_index: 0,
            device_name: None,
            spilling,
        }
    }

    // --- format_ps_table ---

    #[test]
    fn format_ps_table_empty_prints_header_only() {
        let s = format_ps_table(&[]);
        // Header line ends with newline; widths default to header lengths.
        assert_eq!(s, "PID  NAME  VRAM  SHARED  DEVICE  SPILL\n");
    }

    #[test]
    fn format_ps_table_single_row() {
        let r = row(
            12345,
            Some("python.exe"),
            8_589_934_592, // 8 GiB
            0,
            Some("RTX 5060 Ti"),
        );
        let s = format_ps_table(&[r]);
        let expected = "PID    NAME        VRAM     SHARED  DEVICE       SPILL\n\
                        12345  python.exe  8.0 GiB  0 MiB   RTX 5060 Ti  ?    \n";
        assert_eq!(s, expected);
    }

    #[test]
    fn format_ps_table_protected_name_renders_question_mark() {
        // Column widths: PID=3 (header), NAME=4 (header), VRAM=7
        // ("256 MiB"), SHARED=6 (header), DEVICE=11 ("RTX 5060 Ti"),
        // SPILL=5 (header — "?" is shorter). Two-space separators.
        let r = row(99, Some("?"), 268_435_456, 0, Some("RTX 5060 Ti"));
        let s = format_ps_table(&[r]);
        let expected = "PID  NAME  VRAM     SHARED  DEVICE       SPILL\n\
                        99   ?     256 MiB  0 MiB   RTX 5060 Ti  ?    \n";
        assert_eq!(s, expected);
    }

    #[test]
    fn format_ps_table_missing_name_renders_question_mark() {
        // Missing name (None) renders identically to the protected `?`
        // case — both go through the `unwrap_or("?")` path.
        let r = row(99, None, 268_435_456, 0, Some("RTX 5060 Ti"));
        let s = format_ps_table(&[r]);
        let expected = "PID  NAME  VRAM     SHARED  DEVICE       SPILL\n\
                        99   ?     256 MiB  0 MiB   RTX 5060 Ti  ?    \n";
        assert_eq!(s, expected);
    }

    #[test]
    fn format_ps_table_spill_column_renders_spill_no_and_unknown() {
        let rows = [
            row_spilling(1, Some("a.exe"), Some(true)),
            row_spilling(2, Some("b.exe"), Some(false)),
            row_spilling(3, Some("c.exe"), None),
        ];
        let s = format_ps_table(&rows);
        assert!(s.contains("a.exe  0 MiB  0 MiB   GPU 0   SPILL"));
        assert!(s.contains("b.exe  0 MiB  0 MiB   GPU 0   no   "));
        assert!(s.contains("c.exe  0 MiB  0 MiB   GPU 0   ?    "));
    }

    #[test]
    fn format_ps_table_falls_back_to_gpu_n_when_no_device_name() {
        let r = row(99, Some("python.exe"), 268_435_456, 3, None);
        let s = format_ps_table(&[r]);
        assert!(s.contains("python.exe  256 MiB  0 MiB   GPU 3"));
    }

    #[test]
    fn format_ps_table_shared_column_renders_nonzero_bytes() {
        // A genuinely spilling row: 16 GiB dedicated commit, 2 GiB
        // resident shared. The SHARED cell goes through the same
        // format_vram path as VRAM.
        let r = row_shared(
            77,
            Some("py.exe"),
            16 * 1024 * 1024 * 1024,
            2 * 1024 * 1024 * 1024,
        );
        let s = format_ps_table(&[r]);
        assert!(s.contains("16.0 GiB  2.0 GiB"));
    }

    // --- format_ps_json ---

    #[test]
    fn format_ps_json_empty() {
        assert_eq!(format_ps_json(&[]), "[]\n");
    }

    #[test]
    fn format_ps_json_single_row() {
        let r = row(
            12345,
            Some("python.exe"),
            8 * 1_048_576,
            0,
            Some("RTX 5060 Ti"),
        );
        let s = format_ps_json(&[r]);
        assert_eq!(
            s,
            "[{\"pid\":12345,\"name\":\"python.exe\",\"used_bytes\":8388608,\"shared_used_bytes\":0,\"device_index\":0,\"device_name\":\"RTX 5060 Ti\",\"spilling\":null}]\n"
        );
    }

    #[test]
    fn format_ps_json_null_name() {
        let r = row(42, None, 0, 0, None);
        let s = format_ps_json(&[r]);
        assert_eq!(
            s,
            "[{\"pid\":42,\"name\":null,\"used_bytes\":0,\"shared_used_bytes\":0,\"device_index\":0,\"device_name\":null,\"spilling\":null}]\n"
        );
    }

    #[test]
    fn format_ps_json_two_rows_comma_separated() {
        let a = row(1, Some("a.exe"), 1_048_576, 0, Some("GPU"));
        let b = row(2, Some("b.exe"), 2_097_152, 0, Some("GPU"));
        let s = format_ps_json(&[a, b]);
        assert_eq!(
            s,
            "[{\"pid\":1,\"name\":\"a.exe\",\"used_bytes\":1048576,\"shared_used_bytes\":0,\"device_index\":0,\"device_name\":\"GPU\",\"spilling\":null},\
             {\"pid\":2,\"name\":\"b.exe\",\"used_bytes\":2097152,\"shared_used_bytes\":0,\"device_index\":0,\"device_name\":\"GPU\",\"spilling\":null}]\n"
        );
    }

    #[test]
    fn format_ps_json_spilling_true_false_null() {
        let rows = [
            row_spilling(1, Some("a.exe"), Some(true)),
            row_spilling(2, Some("b.exe"), Some(false)),
            row_spilling(3, Some("c.exe"), None),
        ];
        let s = format_ps_json(&rows);
        assert!(s.contains(r#""pid":1,"name":"a.exe","used_bytes":0,"shared_used_bytes":0,"device_index":0,"device_name":null,"spilling":true"#));
        assert!(s.contains(r#""pid":2,"name":"b.exe","used_bytes":0,"shared_used_bytes":0,"device_index":0,"device_name":null,"spilling":false"#));
        assert!(s.contains(r#""pid":3,"name":"c.exe","used_bytes":0,"shared_used_bytes":0,"device_index":0,"device_name":null,"spilling":null"#));
    }

    #[test]
    fn format_ps_json_nonzero_shared_bytes() {
        let r = row_shared(7, Some("py.exe"), 1_048_576, 424_242);
        let s = format_ps_json(&[r]);
        assert!(s.contains("\"used_bytes\":1048576,\"shared_used_bytes\":424242,"));
    }

    #[test]
    fn format_ps_json_escapes_quotes_in_name() {
        let r = row(1, Some(r#"weird"name"#), 0, 0, None);
        let s = format_ps_json(&[r]);
        assert!(s.contains(r#""name":"weird\"name""#));
    }

    // --- format_ps_summary (stderr count line) ---

    /// Build `n` `PsRow`s with resolved names — used by tests that
    /// focus on count and filter clauses, not the protected-count
    /// parenthetical (which is exercised separately).
    fn unprotected_rows(n: u32) -> Vec<PsRow> {
        (0..n)
            .map(|i| row(1000 + i, Some("test.exe"), 0, 0, None))
            .collect()
    }

    /// Build `n` `PsRow`s with `name: None` — used to exercise the
    /// protected-count parenthetical.
    fn protected_rows(n: u32) -> Vec<PsRow> {
        (0..n).map(|i| row(2000 + i, None, 0, 0, None)).collect()
    }

    #[test]
    fn format_ps_summary_zero_no_filters() {
        assert_eq!(
            format_ps_summary(&unprotected_rows(0), None, None, None),
            "0 GPU processes found."
        );
    }

    #[test]
    fn format_ps_summary_one_no_filters() {
        // Singular noun, no filter clause. `used_bytes: 0` rows still
        // get a committed-total parenthetical (the figure is 0 MiB —
        // honest, even when uninteresting).
        assert_eq!(
            format_ps_summary(&unprotected_rows(1), None, None, None),
            "1 GPU process found (0 MiB committed total)."
        );
    }

    #[test]
    fn format_ps_summary_many_no_filters() {
        assert_eq!(
            format_ps_summary(&unprotected_rows(7), None, None, None),
            "7 GPU processes found (0 MiB committed total)."
        );
    }

    #[test]
    fn format_ps_summary_with_pid_filter() {
        // Zero rows → no parenthetical at all (committed-total
        // elides; the filter clause still appears).
        assert_eq!(
            format_ps_summary(&unprotected_rows(0), Some(12345), None, None),
            "0 GPU processes found matching pid=12345."
        );
    }

    #[test]
    fn format_ps_summary_with_device_filter() {
        assert_eq!(
            format_ps_summary(&unprotected_rows(2), None, Some(0), None),
            "2 GPU processes found matching device=0 (0 MiB committed total)."
        );
    }

    #[test]
    fn format_ps_summary_with_both_filters() {
        assert_eq!(
            format_ps_summary(&unprotected_rows(1), Some(99), Some(1), None),
            "1 GPU process found matching pid=99 device=1 (0 MiB committed total)."
        );
    }

    #[test]
    fn format_ps_summary_with_min_filter() {
        assert_eq!(
            format_ps_summary(&unprotected_rows(0), None, None, Some(50 * 1024 * 1024)),
            "0 GPU processes found matching min=50 MiB."
        );
    }

    #[test]
    fn format_ps_summary_sub_mib_min_filter_is_not_misreported_as_zero() {
        // Regression: format_ps_summary used to echo --min through
        // format_vram, so a genuine 512 KiB filter read back as
        // "min=0 MiB" — indistinguishable from the documented --min 0
        // no-op, even though rows were actually being hidden.
        let s = format_ps_summary(&unprotected_rows(0), None, None, Some(512 * 1024));
        assert_eq!(s, "0 GPU processes found matching min=512 KiB.");
    }

    #[test]
    fn format_ps_summary_with_all_three_filters() {
        assert_eq!(
            format_ps_summary(
                &unprotected_rows(1),
                Some(99),
                Some(1),
                Some(50 * 1024 * 1024)
            ),
            "1 GPU process found matching pid=99 device=1 min=50 MiB (0 MiB committed total)."
        );
    }

    // -- committed-total parenthetical (non-zero VRAM) --

    #[test]
    fn format_ps_summary_with_committed_total_gib() {
        // 3 rows at 4 GiB each → 12 GiB committed total, formatted
        // with one decimal place to match `format_vram`'s GiB output.
        const FOUR_GIB: u64 = 4 * 1024 * 1024 * 1024;
        let rows = vec![
            row(1001, Some("a.exe"), FOUR_GIB, 0, None),
            row(1002, Some("b.exe"), FOUR_GIB, 0, None),
            row(1003, Some("c.exe"), FOUR_GIB, 0, None),
        ];
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "3 GPU processes found (12.0 GiB committed total)."
        );
    }

    #[test]
    fn format_ps_summary_with_committed_total_mib() {
        // 2 rows at 256 MiB each → 512 MiB, below 1 GiB threshold,
        // formatter renders as MiB.
        const QUARTER_GIB: u64 = 256 * 1024 * 1024;
        let rows = vec![
            row(1001, Some("a.exe"), QUARTER_GIB, 0, None),
            row(1002, Some("b.exe"), QUARTER_GIB, 0, None),
        ];
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "2 GPU processes found (512 MiB committed total)."
        );
    }

    // -- protected-count parenthetical --

    #[test]
    fn format_ps_summary_one_protected_appends_parenthetical() {
        let mut rows = unprotected_rows(3);
        rows.extend(protected_rows(1));
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "4 GPU processes found (0 MiB committed total; 1 protected — re-run elevated for names)."
        );
    }

    #[test]
    fn format_ps_summary_many_protected_appends_parenthetical() {
        let mut rows = unprotected_rows(28);
        rows.extend(protected_rows(4));
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "32 GPU processes found (0 MiB committed total; 4 protected — re-run elevated for names)."
        );
    }

    #[test]
    fn format_ps_summary_all_protected() {
        let rows = protected_rows(3);
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "3 GPU processes found (0 MiB committed total; 3 protected — re-run elevated for names)."
        );
    }

    #[test]
    fn format_ps_summary_zero_protected_elides_protected_part_keeps_total() {
        // No protected rows → no `M protected …` clause, but the
        // committed-total parenthetical still appears.
        assert_eq!(
            format_ps_summary(&unprotected_rows(5), None, None, None),
            "5 GPU processes found (0 MiB committed total)."
        );
    }

    #[test]
    fn format_ps_summary_protected_with_filters_both_appear() {
        let mut rows = unprotected_rows(2);
        rows.extend(protected_rows(1));
        assert_eq!(
            format_ps_summary(&rows, Some(42), Some(0), None),
            "3 GPU processes found matching pid=42 device=0 (0 MiB committed total; 1 protected — re-run elevated for names)."
        );
    }

    #[test]
    fn format_ps_summary_bracket_protected_string_counts_as_protected() {
        // Windows-only `[protected]` synthetic name (the
        // Toolhelp32Snapshot fallback itself could not be taken) counts
        // toward the same "re-run elevated" hint as a bare `name: None`
        // row, even though `name` is `Some` here.
        let mut rows = unprotected_rows(2);
        rows.push(row(3000, Some("[protected]"), 0, 0, None));
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "3 GPU processes found (0 MiB committed total; 1 protected — re-run elevated for names)."
        );
    }

    #[test]
    fn format_ps_summary_nvidia_smi_question_mark_counts_as_protected() {
        // Pre-existing (not v0.2.8-introduced) case: the pre-WDDM-2.0
        // `nvidia-smi` fallback writes a literal `"?"` name string rather
        // than `None` for a row it couldn't identify. This carries the
        // same "might resolve under elevation" meaning as `None`/
        // `[protected]` and must count toward the hint too — previously
        // it silently didn't, understating the count exactly the way
        // `[exited]` would have overstated it.
        let mut rows = unprotected_rows(2);
        rows.push(row(3002, Some("?"), 0, 0, None));
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "3 GPU processes found (0 MiB committed total; 1 protected — re-run elevated for names)."
        );
    }

    #[test]
    fn format_ps_summary_bracket_exited_string_does_not_count_as_protected() {
        // `[exited]` means the process was already gone by the time of
        // the name lookup — elevation would not have helped, so it must
        // NOT inflate the protected count (this is the exact
        // overstatement the v0.2.8 dogfooding report flagged).
        let mut rows = unprotected_rows(2);
        rows.push(row(3001, Some("[exited]"), 0, 0, None));
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "3 GPU processes found (0 MiB committed total)."
        );
    }

    #[test]
    fn format_ps_summary_kernel_bracket_does_not_count_as_protected() {
        // `[kernel]` (PID 4) has no executable image to resolve
        // regardless of privilege — unchanged pre-v0.2.8 behaviour,
        // re-asserted here alongside the new bracket-counting tests.
        let mut rows = unprotected_rows(2);
        rows.push(row(4, Some("[kernel]"), 0, 0, None));
        assert_eq!(
            format_ps_summary(&rows, None, None, None),
            "3 GPU processes found (0 MiB committed total)."
        );
    }

    // --- ps_row_comparator / SortKey ---

    /// Like [`row`] but with an explicit `shared_used_bytes`, needed to
    /// exercise `SortKey::Shared` / `SortKey::Total`.
    fn row_full(pid: u32, name: &str, used_bytes: u64, shared_used_bytes: u64) -> PsRow {
        PsRow {
            pid,
            name: Some(name.to_owned()),
            used_bytes,
            shared_used_bytes,
            device_index: 0,
            device_name: None,
            spilling: None,
        }
    }

    fn sorted_pids(rows: &mut [PsRow], key: SortKey) -> Vec<u32> {
        rows.sort_by(ps_row_comparator(key));
        rows.iter().map(|r| r.pid).collect()
    }

    #[test]
    fn ps_row_comparator_dedicated_descending() {
        let mut rows = vec![
            row_full(1, "a.exe", 1_000, 9_000),
            row_full(2, "b.exe", 5_000, 0),
            row_full(3, "c.exe", 3_000, 0),
        ];
        assert_eq!(sorted_pids(&mut rows, SortKey::Dedicated), vec![2, 3, 1]);
    }

    #[test]
    fn ps_row_comparator_shared_descending() {
        let mut rows = vec![
            row_full(1, "a.exe", 1_000, 9_000),
            row_full(2, "b.exe", 5_000, 0),
            row_full(3, "c.exe", 3_000, 2_000),
        ];
        assert_eq!(sorted_pids(&mut rows, SortKey::Shared), vec![1, 3, 2]);
    }

    #[test]
    fn ps_row_comparator_total_descending_differs_from_dedicated_and_shared() {
        // pid 1: total 10_000 (highest) but neither dedicated- nor
        // shared-highest alone — only `total` puts it first.
        let mut rows = vec![
            row_full(1, "a.exe", 4_000, 6_000),
            row_full(2, "b.exe", 8_000, 0),
            row_full(3, "c.exe", 0, 7_000),
        ];
        assert_eq!(sorted_pids(&mut rows, SortKey::Total), vec![1, 2, 3]);
        // Confirms neither single-field key would have produced this order.
        assert_eq!(sorted_pids(&mut rows, SortKey::Dedicated), vec![2, 1, 3]);
        assert_eq!(sorted_pids(&mut rows, SortKey::Shared), vec![3, 1, 2]);
    }

    #[test]
    fn ps_row_comparator_tie_break_identical_across_keys() {
        // Two rows tied on every numeric field: every key must fall
        // through to the same name-then-PID tie-break.
        let mut rows = vec![
            row_full(20, "b.exe", 1_000, 1_000),
            row_full(10, "a.exe", 1_000, 1_000),
        ];
        for key in [SortKey::Dedicated, SortKey::Shared, SortKey::Total] {
            assert_eq!(sorted_pids(&mut rows, key), vec![10, 20], "key {key:?}");
        }
    }

    #[test]
    fn ps_row_comparator_total_saturates_instead_of_overflowing() {
        // Pathological but must not panic: plain `+` on two `u64::MAX`
        // values panics in debug builds (and silently wraps in
        // release); `saturating_add` does neither and still orders
        // this row correctly ahead of a small, unambiguous total.
        let mut rows = vec![
            row_full(1, "a.exe", u64::MAX, u64::MAX),
            row_full(2, "b.exe", 1_000, 0),
        ];
        assert_eq!(sorted_pids(&mut rows, SortKey::Total), vec![1, 2]);
    }
}
