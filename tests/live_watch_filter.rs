// SPDX-License-Identifier: MIT OR Apache-2.0

//! Live end-to-end test for `hmn watch --follow-new --filter` against a
//! real `spillforge` run — the regression case from the candle-mi
//! dogfooding report that asked for the filter
//! (`docs/dogfooding-feedbacks/dogfooding-watch-filter-by-identity.md`):
//! there, `--follow-new --top 3` recorded 73.9% desktop rows; with
//! `--filter`, every recorded row must be the workload.
//!
//! Also checks the v0.2.12 `start` record: the first `--json` line, and
//! the one that says how the capture was selected.
//!
//! Same gate and prerequisite as `tests/live_watch.rs` (`windows + pdh +
//! cli`; `tools/spillforge/target/release/spillforge.exe` prebuilt):
//!
//! ```sh
//! cargo build --release --manifest-path tools/spillforge/Cargo.toml
//! cargo test --features cli,pdh --test live_watch_filter -- --ignored
//! ```

#![cfg(all(windows, feature = "pdh", feature = "cli"))]

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Path to the spillforge fixture — see `tests/live_watch.rs`'s copy of
/// this helper for the full rationale, and `tests/live_watch_follow_new.rs`
/// for why it is kept duplicated rather than shared.
fn spillforge_path() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tools/spillforge/target/release/spillforge.exe");
    assert!(
        path.exists(),
        "spillforge fixture not built — run: cargo build --release \
         --manifest-path tools/spillforge/Cargo.toml (path checked: {})",
        path.display()
    );
    path
}

/// End-to-end acceptance test: `hmn watch --follow-new --filter spillforge
/// --top 3` attached to a desktop that holds other GPU processes, while a
/// real `spillforge` run comes and goes. Every `sample` row and every
/// closing `per_pid` entry must be `spillforge.exe` — the desktop that
/// `--top 3` alone would have recorded is excluded — and the first line
/// must be the `start` record naming the filter.
#[test]
#[ignore = "requires Windows + WDDM 2.0+ GPU and a prebuilt spillforge fixture"]
#[allow(clippy::expect_used, clippy::panic)] // test-only
fn hmn_watch_filter_records_only_the_named_workload() {
    let watch = Command::new(env!("CARGO_BIN_EXE_hmn"))
        .args([
            "watch",
            "--follow-new",
            "--top",
            "3",
            "--filter",
            "SpillForge", // case-insensitive: matches spillforge.exe
            "--interval",
            "2s",
            "--duration",
            "40s",
            "--json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn hmn watch");

    // Let the watch attach (with nothing matching yet) before the workload
    // starts, so spillforge is a genuine "entered" transition.
    std::thread::sleep(Duration::from_secs(2));
    let status = Command::new(spillforge_path())
        .args(["8", "10"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run spillforge");
    assert!(status.success(), "spillforge exited with {status:?}");

    let output = watch
        .wait_with_output()
        .expect("failed to wait on hmn watch");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = stdout.lines().collect();

    let first = lines.first().copied().unwrap_or_default();
    assert!(
        first.starts_with(r#"{"kind":"start","t_ms":0,"#),
        "first --json line must be the start record, got: {first}"
    );
    assert!(
        first.contains(r#""selection":{"mode":"follow_new","pids":[],"top":3,"filters":["SpillForge"],"min_bytes":null}"#),
        "start record must carry the selection as given, got: {first}"
    );

    let samples: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| l.contains(r#""kind":"sample""#))
        .collect();
    assert!(
        !samples.is_empty(),
        "expected spillforge to be sampled at least once; stderr:\n{stderr}"
    );
    for s in &samples {
        assert!(
            s.contains(r#""name":"spillforge.exe""#),
            "every sample must be the filtered workload, got: {s}"
        );
    }

    let summary = lines
        .iter()
        .copied()
        .find(|l| l.contains(r#""kind":"summary""#))
        .unwrap_or_else(|| panic!("no summary line in hmn watch --json output:\n{stdout}"));
    let per_pid_names = summary.matches(r#""name":"#).count();
    let spillforge_names = summary.matches(r#""name":"spillforge.exe""#).count();
    assert!(
        spillforge_names >= 1 && spillforge_names == per_pid_names,
        "every per_pid entry must be spillforge.exe, got: {summary}"
    );

    assert!(
        stderr.contains(r#"among names containing "SpillForge" (case-insensitive)"#),
        "the stderr header must announce the filter; stderr:\n{stderr}"
    );
}
