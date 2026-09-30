// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test fixtures shared by more than one subcommand module's tests
//! (compiled only under `cfg(test)`).

#[cfg(feature = "test-helpers")]
use std::time::Duration;

#[cfg(feature = "test-helpers")]
use hypomnesis::{GpuProcessEntry, SpillReport};

use crate::ps::PsRow;
use crate::watch::WatchPidSummary;

/// A `hmn ps` row with zero shared bytes and no spill verdict — the
/// baseline fixture for the table, JSON and comparator tests, and for
/// `hmn watch`'s top-N selection (which shares `hmn ps`'s comparator).
pub fn row(
    pid: u32,
    name: Option<&str>,
    used_bytes: u64,
    device_index: u32,
    device_name: Option<&str>,
) -> PsRow {
    PsRow {
        pid,
        name: name.map(str::to_owned),
        used_bytes,
        shared_used_bytes: 0,
        device_index,
        device_name: device_name.map(str::to_owned),
        spilling: None,
        paged: None,
        shared_share: None,
    }
}

/// A library-side process entry, as `gpu_processes` returns it — for
/// `hmn watch`'s sampling and selection and `hmn ps`'s row filters.
/// Built through the `test-helpers` builder: `GpuProcessEntry` is
/// `#[non_exhaustive]`.
#[cfg(feature = "test-helpers")]
pub fn entry(
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

/// A measurable report with two spill episodes (one closed, one still
/// open at report time), for `hmn spill`'s and `hmn watch`'s report
/// formatters. Built through the `test-helpers` builder: `SpillReport` is
/// `#[non_exhaustive]`, so the binary cannot struct-literal one — see
/// `SpillReportBuilder`'s docs.
#[cfg(feature = "test-helpers")]
pub fn spilling_report() -> SpillReport {
    const GIB: u64 = 1024 * 1024 * 1024;
    SpillReport::builder()
        .measurable(true)
        .observations(200)
        .peak_dedicated_bytes(16 * GIB)
        .dedicated_limit_bytes(16 * GIB)
        .peak_shared_bytes(4 * GIB + 200 * 1024 * 1024) // 4.2 GiB
        .baseline_shared_bytes(300 * 1024 * 1024)
        .episode(
            "+12.4s",
            Some("+15.5s"),
            3 * GIB,
            31,
            Duration::from_millis(3_100),
        )
        .episode("+20.0s", None, 4 * GIB, 67, Duration::from_millis(6_700))
        .build()
}

/// One `hmn watch` closing-summary `per_pid` entry, for the watch summary
/// formatters and the `SpillReport` JSON key-parity tests.
pub fn pid_summary(
    pid: u32,
    name: Option<&str>,
    baseline_used: u64,
    peak_used: u64,
    baseline_shared: u64,
    peak_shared: u64,
) -> WatchPidSummary {
    WatchPidSummary {
        pid,
        name: name.map(str::to_owned),
        baseline_used_bytes: baseline_used,
        peak_used_bytes: peak_used,
        baseline_shared_bytes: baseline_shared,
        peak_shared_bytes: peak_shared,
        paged: None,
    }
}
