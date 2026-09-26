// SPDX-License-Identifier: MIT OR Apache-2.0

//! `hmn spill -- <command>`: run a command under a `SpillTracker` and
//! report its spill episodes. Also owns the `SpillReport` text and JSON
//! renderers `hmn watch` reuses for its closing summary, so the two
//! subcommands can never disagree on that shape.

use std::fmt::Write as _;
use std::time::Duration;

use hypomnesis::{SpillEpisode, SpillReport, SpillTracker};

use crate::format::{duration_ms, format_secs, format_vram, json_escape};

/// Run the `spill` subcommand: spawn the wrapped command with
/// inherited stdio, poll a [`SpillTracker`] every `interval_ms` until
/// the child exits, print the report (stderr human block; optional
/// stdout JSON), and pass the child's exit code through.
///
/// Measurement failures never stop the workload: a tracker that fails
/// to construct produces a stderr warning and the child runs
/// unmeasured. Only spawn/wait failures — where there is no child
/// outcome to pass through — return `hmn`'s own `FAILURE`.
pub fn run_spill(
    interval_ms: u64,
    device: u32,
    json: bool,
    command: &[String],
) -> std::process::ExitCode {
    // clap's `required = true` on the trailing arg makes an empty
    // command unreachable in practice; belt-and-braces for direct calls.
    let Some((program, args)) = command.split_first() else {
        eprintln!("hmn: spill requires a command to run (after `--`)");
        return std::process::ExitCode::FAILURE;
    };

    let mut tracker = match SpillTracker::new(device) {
        Ok(t) => Some(t),
        Err(e) => {
            eprintln!("hmn: spill tracking unavailable ({e}); running command unmeasured");
            None
        }
    };

    let mut child = match std::process::Command::new(program).args(args).spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("hmn: failed to spawn {program:?}: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let run_start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if let Some(t) = tracker.as_mut() {
                    t.observe(format!("+{:.1}s", run_start.elapsed().as_secs_f64()));
                }
                std::thread::sleep(Duration::from_millis(interval_ms));
            }
            Err(e) => {
                eprintln!("hmn: failed to wait on wrapped command: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
    };

    // One final observation at exit so short-lived commands get at
    // least one sample, then the report.
    if let Some(t) = tracker.as_mut() {
        t.observe(format!("+{:.1}s", run_start.elapsed().as_secs_f64()));
    }
    if let Some(report) = tracker.map(SpillTracker::into_report) {
        if json {
            print!("{}", format_spill_json(Some(&report)));
        }
        if report.measurable {
            eprint!("{}", format_spill_report(&report));
        } else {
            eprintln!("hmn spill: spill not measurable on this platform");
        }
    } else {
        // Tracker construction failed (already warned above).
        if json {
            print!("{}", format_spill_json(None));
        }
        eprintln!("hmn spill: spill not measurable (no tracker)");
    }

    std::process::ExitCode::from(exit_code_byte(status.code()))
}

/// Map a child's `ExitStatus::code()` to the byte `hmn` exits with.
///
/// `0..=255` passes through exactly. Codes outside that range —
/// negative Windows `NTSTATUS` values (e.g. `0xC0000005` as `i32`),
/// or >255 — map to `1` rather than being bit-truncated: truncation
/// could turn a failure like 256 into a false success. `None` (child
/// killed by a signal on Unix) also maps to `1`.
fn exit_code_byte(code: Option<i32>) -> u8 {
    code.map_or(1, |c| u8::try_from(c).unwrap_or(1))
}

/// Format the human-readable spill report block printed to stderr, under
/// a caller-chosen `prefix` (e.g. `"hmn spill"`, `"hmn watch"`).
///
/// Three aligned lines, continuation lines indented to `prefix.len() + 2`
/// spaces (matching `"<prefix>: "`'s width). The dedicated line elides
/// its `/ capacity` suffix when the capacity is unknown
/// (`dedicated_limit_bytes == 0`); the episodes line collapses to `no
/// spill observed` when no episode was recorded. The `first ... into
/// run` fragment reuses the episode start label, which callers stamp as
/// elapsed time (`"+12.4s"`).
pub fn format_spill_report_with_prefix(prefix: &str, report: &SpillReport) -> String {
    let mut out = String::new();
    let indent = " ".repeat(prefix.len() + 2);
    let limit_suffix = if report.dedicated_limit_bytes > 0 {
        format!(" / {}", format_vram(report.dedicated_limit_bytes))
    } else {
        String::new()
    };
    let _ = writeln!(
        out,
        "{prefix}: peak dedicated {}{limit_suffix}",
        format_vram(report.peak_dedicated_bytes)
    );
    let _ = writeln!(
        out,
        "{indent}peak shared    {} (baseline {})",
        format_vram(report.peak_shared_bytes),
        format_vram(report.baseline_shared_bytes)
    );
    if report.spilled() {
        let total = format_secs(report.total_spill_duration());
        let longest = report
            .longest_episode()
            .map_or_else(String::new, |e| format_secs(e.duration));
        let first = report.first_spill_label().unwrap_or("?");
        let _ = writeln!(
            out,
            "{indent}episodes       {} — total {total}, longest {longest}, first {first} into run",
            report.episodes.len()
        );
    } else {
        let _ = writeln!(out, "{indent}episodes       0 — no spill observed");
    }
    out
}

/// [`format_spill_report_with_prefix`] under the `hmn spill` prefix —
/// the report block `hmn spill -- <command>` prints to stderr on exit.
fn format_spill_report(report: &SpillReport) -> String {
    format_spill_report_with_prefix("hmn spill", report)
}

/// Write a [`SpillReport`]'s episodes as a JSON array (`[...]`, no
/// trailing content) into `out`. Called only by
/// [`write_spill_report_fields`], so the episode shape and escaping
/// ([`json_escape`]) are identical across both subcommands' `--json`
/// output.
fn write_episodes_json(out: &mut String, episodes: &[SpillEpisode]) {
    out.push('[');
    for (i, ep) in episodes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let end_label = ep.end_label.as_deref().map_or_else(
            || String::from("null"),
            |l| format!("\"{}\"", json_escape(l)),
        );
        let _ = write!(
            out,
            r#"{{"start_label":"{}","end_label":{end_label},"peak_shared_bytes":{},"observations":{},"duration_ms":{}}}"#,
            json_escape(&ep.start_label),
            ep.peak_shared_bytes,
            ep.observations,
            duration_ms(ep.duration),
        );
    }
    out.push(']');
}

/// Write the adapter-level [`SpillReport`] JSON fields — `"measurable"`
/// through `"episodes":[...]`, without surrounding braces — into `out`.
///
/// The one spelling of that wire contract: [`format_spill_json`] wraps
/// it in braces for `hmn spill --json`, and
/// [`format_watch_summary_json`](crate::watch::format_watch_summary_json) embeds it after its `"kind"` tag for
/// `hmn watch --json`. `None` (no `SpillTracker` could be constructed)
/// writes the all-zeros `"measurable":false` shape, so scripted
/// consumers parse one shape either way. Hand-rolled (no `serde` dep —
/// same policy as `ps::format_ps_json`); labels are escaped via
/// [`json_escape`]; durations are integer milliseconds.
pub fn write_spill_report_fields(out: &mut String, report: Option<&SpillReport>) {
    let _ = write!(
        out,
        r#""measurable":{},"spilled":{},"observations":{},"baseline_shared_bytes":{},"peak_shared_bytes":{},"peak_dedicated_bytes":{},"dedicated_limit_bytes":{},"total_spill_duration_ms":{},"episodes":"#,
        report.is_some_and(|r| r.measurable),
        report.is_some_and(SpillReport::spilled),
        report.map_or(0, |r| r.observations),
        report.map_or(0, |r| r.baseline_shared_bytes),
        report.map_or(0, |r| r.peak_shared_bytes),
        report.map_or(0, |r| r.peak_dedicated_bytes),
        report.map_or(0, |r| r.dedicated_limit_bytes),
        report.map_or(0, |r| duration_ms(r.total_spill_duration())),
    );
    write_episodes_json(out, report.map_or(&[], |r| &r.episodes));
}

/// Format a [`SpillReport`] — or, for `None`, the all-zeros
/// `"measurable":false` fallback — as a single newline-terminated JSON
/// object for `hmn spill --json`. See [`write_spill_report_fields`].
#[must_use]
fn format_spill_json(report: Option<&SpillReport>) -> String {
    let mut out = String::from("{");
    write_spill_report_fields(&mut out, report);
    out.push_str("}\n");
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
    use crate::test_support::pid_summary;
    #[cfg(feature = "test-helpers")]
    use crate::test_support::spilling_report;
    use crate::watch::format_watch_summary_json;

    // --- exit_code_byte (spill exit-code pass-through) ---

    #[test]
    fn exit_code_byte_zero_passes_through() {
        assert_eq!(exit_code_byte(Some(0)), 0);
    }

    #[test]
    fn exit_code_byte_passthrough_255() {
        assert_eq!(exit_code_byte(Some(7)), 7);
        assert_eq!(exit_code_byte(Some(255)), 255);
    }

    #[test]
    fn exit_code_byte_negative_is_one() {
        // Windows NTSTATUS codes surface as negative i32 (e.g. an
        // access violation 0xC0000005); never truncate.
        assert_eq!(exit_code_byte(Some(-1_073_741_819)), 1);
        assert_eq!(exit_code_byte(Some(-1)), 1);
    }

    #[test]
    fn exit_code_byte_overflow_is_one() {
        // Truncating 256 to u8 would yield 0 — a false success.
        assert_eq!(exit_code_byte(Some(256)), 1);
        assert_eq!(exit_code_byte(Some(i32::MAX)), 1);
    }

    #[test]
    fn exit_code_byte_none_is_one() {
        // Signal-killed child on Unix: no exit code.
        assert_eq!(exit_code_byte(None), 1);
    }

    // --- spill report formatting (fixtures via the test-helpers
    //     builder: SpillReport is #[non_exhaustive], so the binary
    //     cannot struct-literal one — see SpillReportBuilder docs) ---

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_spill_report_episodes_line() {
        let s = format_spill_report(&spilling_report());
        let expected = "hmn spill: peak dedicated 16.0 GiB / 16.0 GiB\n           peak shared    4.2 GiB (baseline 300 MiB)\n           episodes       2 — total 9.8s, longest 6.7s, first +12.4s into run\n";
        assert_eq!(s, expected);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_spill_report_no_episodes() {
        const GIB: u64 = 1024 * 1024 * 1024;
        let report = SpillReport::builder()
            .measurable(true)
            .observations(50)
            .peak_dedicated_bytes(14 * GIB)
            .dedicated_limit_bytes(16 * GIB)
            .peak_shared_bytes(140 * 1024 * 1024)
            .baseline_shared_bytes(134 * 1024 * 1024)
            .build();
        let s = format_spill_report(&report);
        assert!(s.contains("episodes       0 — no spill observed"));
        assert!(s.contains("peak dedicated 14.0 GiB / 16.0 GiB"));
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_spill_report_unknown_limit_elides_suffix() {
        let report = SpillReport::builder()
            .measurable(true)
            .peak_dedicated_bytes(1024 * 1024 * 1024)
            .build();
        let s = format_spill_report(&report);
        assert!(s.contains("peak dedicated 1.0 GiB\n"));
        assert!(!s.contains(" / "));
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_spill_json_shape() {
        let s = format_spill_json(Some(&spilling_report()));
        assert!(s.starts_with("{\"measurable\":true,\"spilled\":true,\"observations\":200,"));
        assert!(s.contains("\"total_spill_duration_ms\":9800,"));
        assert!(s.contains("\"episodes\":[{\"start_label\":\"+12.4s\",\"end_label\":\"+15.5s\","));
        assert!(s.ends_with("]}\n"));
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_spill_json_null_end_label() {
        let s = format_spill_json(Some(&spilling_report()));
        assert!(s.contains("\"start_label\":\"+20.0s\",\"end_label\":null,"));
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_spill_json_escapes_labels() {
        let report = SpillReport::builder()
            .measurable(true)
            .episode("weird\"label", None, 0, 1, Duration::ZERO)
            .build();
        let s = format_spill_json(Some(&report));
        assert!(s.contains(r#""start_label":"weird\"label""#));
    }

    #[test]
    fn spill_json_unmeasurable_is_the_all_zeros_shape() {
        // The hard-error fallback (no SpillTracker at all), pinned
        // byte-for-byte: this is the exact object the pre-v0.2.12
        // `SPILL_JSON_UNMEASURABLE` constant emitted, so scripted
        // consumers see no change now that it is generated.
        assert_eq!(
            format_spill_json(None),
            "{\"measurable\":false,\"spilled\":false,\"observations\":0,\
             \"baseline_shared_bytes\":0,\"peak_shared_bytes\":0,\
             \"peak_dedicated_bytes\":0,\"dedicated_limit_bytes\":0,\
             \"total_spill_duration_ms\":0,\"episodes\":[]}\n"
        );
    }

    // --- format_spill_report_with_prefix (generalized under `hmn watch`) ---

    #[cfg(feature = "test-helpers")]
    #[test]
    fn format_spill_report_with_prefix_watch_matches_spill_shape() {
        // Same content as `format_spill_report`, just under the `hmn
        // watch` prefix — both prefixes are 9 chars, so the continuation
        // indent (11 spaces) is identical.
        let report = spilling_report();
        let s = format_spill_report_with_prefix("hmn watch", &report);
        let expected = "hmn watch: peak dedicated 16.0 GiB / 16.0 GiB\n           peak shared    4.2 GiB (baseline 300 MiB)\n           episodes       2 — total 9.8s, longest 6.7s, first +12.4s into run\n";
        assert_eq!(s, expected);
    }

    // --- write_episodes_json / format_spill_json parity after refactor ---

    #[cfg(feature = "test-helpers")]
    #[test]
    fn write_episodes_json_matches_format_spill_json_episodes() {
        let report = spilling_report();
        let mut direct = String::new();
        write_episodes_json(&mut direct, &report.episodes);
        let whole = format_spill_json(Some(&report));
        assert!(whole.contains(&direct));
    }

    // --- SpillReport JSON field-block parity across every emitter ---
    //
    // The adapter-level `SpillReport` object is emitted by `hmn spill
    // --json` (measurable, and the no-tracker fallback) and embedded in
    // `hmn watch --json`'s closing summary (measurable, and the
    // no-tracker fallback). All four paths now go through
    // `write_spill_report_fields`; these tests pin the whole ordered key
    // list on every one of them, so an output path that stops going
    // through the writer, or a key change nobody meant to make, fails
    // here instead of silently diverging on the wire. (They predate the
    // writer: added while the four paths were still four spellings.)

    /// The canonical ordered key list of the adapter-level `SpillReport`
    /// JSON object — the one place a new field must be added first.
    const SPILL_REPORT_JSON_KEYS: [&str; 9] = [
        "measurable",
        "spilled",
        "observations",
        "baseline_shared_bytes",
        "peak_shared_bytes",
        "peak_dedicated_bytes",
        "dedicated_limit_bytes",
        "total_spill_duration_ms",
        "episodes",
    ];

    /// Top-level keys of one JSON object, in emission order. Tracks
    /// string/escape state and `{`/`[` nesting depth, so keys inside
    /// nested values (`episodes[]`, `per_pid[]`) are excluded. Only as
    /// strict as the hand-rolled emitters it checks — the `cli` feature
    /// deliberately has no `serde` dependency to parse with.
    fn top_level_json_keys(json: &str) -> Vec<&str> {
        let mut keys = Vec::new();
        let mut depth = 0_usize;
        let mut in_string = false;
        let mut escaped = false;
        let mut string_start = 0_usize;
        let mut last_string: Option<&str> = None;
        for (i, c) in json.char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                    last_string = json.get(string_start..i);
                }
                continue;
            }
            match c {
                '"' => {
                    in_string = true;
                    string_start = i + 1;
                }
                '{' | '[' => depth += 1,
                '}' | ']' => depth = depth.saturating_sub(1),
                ':' if depth == 1 => keys.extend(last_string.take()),
                // EXPLICIT: digits, commas, literals and whitespace carry
                // no key information at any depth.
                _ => {}
            }
        }
        keys
    }

    /// A summary object's adapter-level keys: its top-level keys minus
    /// `watch`'s own `kind` tag and `per_pid` array.
    fn spill_report_keys(json: &str) -> Vec<&str> {
        top_level_json_keys(json)
            .into_iter()
            .filter(|k| *k != "kind" && *k != "per_pid")
            .collect()
    }

    #[test]
    fn top_level_json_keys_skips_nested_keys_and_string_contents() {
        let json = r#"{"a":1,"b":[{"c":2}],"d":{"e":"x:\"y\""},"f":"g"}"#;
        assert_eq!(top_level_json_keys(json), ["a", "b", "d", "f"]);
    }

    #[test]
    fn spill_json_unmeasurable_fallbacks_share_the_canonical_keys() {
        // `hmn spill` with no tracker, and `hmn watch` with no tracker.
        assert_eq!(
            spill_report_keys(&format_spill_json(None)),
            SPILL_REPORT_JSON_KEYS
        );
        let watch = format_watch_summary_json(None, &[pid_summary(1, Some("a.exe"), 10, 20, 0, 0)]);
        assert_eq!(spill_report_keys(&watch), SPILL_REPORT_JSON_KEYS);
    }

    #[cfg(feature = "test-helpers")]
    #[test]
    fn spill_json_measurable_emitters_share_the_canonical_keys() {
        // `spilling_report()` carries two episodes and the watch summary
        // a non-empty per_pid, so nested keys are present to be excluded.
        let report = spilling_report();
        assert_eq!(
            spill_report_keys(&format_spill_json(Some(&report))),
            SPILL_REPORT_JSON_KEYS
        );
        let watch = format_watch_summary_json(
            Some(&report),
            &[pid_summary(1, Some("a.exe"), 10, 20, 0, 0)],
        );
        assert_eq!(spill_report_keys(&watch), SPILL_REPORT_JSON_KEYS);
    }
}
