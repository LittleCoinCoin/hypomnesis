// SPDX-License-Identifier: MIT OR Apache-2.0

//! Helpers shared by the live `hmn watch` tests (`tests/live_watch.rs`,
//! `tests/live_watch_follow_new.rs`, `tests/live_watch_filter.rs`), each
//! of which pulls it in with `mod common;`. As a `mod.rs` in a
//! subdirectory, Cargo does not build it as a test binary of its own; it
//! is compiled only inside those files, under their
//! `#![cfg(all(windows, feature = "pdh", feature = "cli"))]` gate.

use std::path::PathBuf;

/// Path to the `spillforge` forced-spill fixture
/// (`tools/spillforge/target/release/spillforge.exe`), built separately
/// per each live test's module docs. Panics with actionable instructions
/// if missing rather than silently skipping — an `#[ignore]`-gated test
/// that's run explicitly is expected to have its prerequisite already
/// satisfied.
pub fn spillforge_path() -> PathBuf {
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

/// The closing `{"kind":"summary",...}` line of an `hmn watch --json`
/// capture. Panics with the whole capture if there is none — a watch
/// that ended without its summary is itself the failure to report.
#[allow(clippy::panic)] // test-only
pub fn summary_line(stdout: &str) -> &str {
    stdout
        .lines()
        .find(|l| l.contains(r#""kind":"summary""#))
        .unwrap_or_else(|| panic!("no summary line in hmn watch --json output:\n{stdout}"))
}
