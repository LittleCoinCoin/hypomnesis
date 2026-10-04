// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end exit-code tests for `hmn ps` that hold on any machine, GPU
//! or not — so, unlike the `live_*` tests, they are not `#[ignore]`d. The
//! one exception is macOS-only and `#[ignore]`d: it needs a usable Metal
//! device and applies a Seatbelt profile with `/usr/bin/sandbox-exec`.
//! `#![cfg(feature = "cli")]`: they run the compiled binary through
//! `env!("CARGO_BIN_EXE_hmn")`, which Cargo only defines when the `hmn`
//! target (`required-features = ["cli"]`) is built.

#![cfg(feature = "cli")]

use std::io::Write as _;
use std::process::Command;

/// Run `hmn` with `args`, returning its exit code and stderr.
#[allow(clippy::expect_used)] // test-only
fn hmn(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_hmn"))
        .args(args)
        .output()
        .expect("failed to run hmn");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `--device` naming a GPU that cannot be listed exits `2` with the
/// reason, where it used to exit `0` with an empty table — a mistyped
/// index read as an idle card. Index 99 cannot be listed anywhere: past
/// the device count where one is known, and without any GPU source
/// otherwise.
#[test]
fn ps_device_out_of_range_exits_2_with_the_reason() {
    let (code, stderr) = hmn(&["ps", "--device", "99"]);
    assert_eq!(code, Some(2), "stderr: {stderr}");
    // A line, not the whole of stderr: `debug-output` builds trace there too.
    assert!(
        stderr
            .lines()
            .any(|l| l.starts_with("hmn: ps failed to query device 99: ")),
        "stderr: {stderr}"
    );
}

/// Whether `stderr` holds a skipped-device line: `hmn ps` without
/// `--device` names each device whose query failed, ending ` (skipped)`.
fn skipped_device_line(stderr: &str) -> bool {
    stderr
        .lines()
        .any(|l| l.starts_with("hmn: ps failed to query device ") && l.ends_with(" (skipped)"))
}

/// Judge an exit code that should be `expected` on a host where every
/// device answers (or none is there to try), and may instead be `2` where
/// a tried device failed, but only together with its skip line: a bare
/// exit `2` never passes. Writes `cli_ps: <label> branch=<branch>` straight
/// to stderr, so the branch taken is in CI's log even for a passing test
/// (`eprintln!` is captured by `libtest` and hidden), and asserts the branch
/// is not `rejected`.
fn accept(label: &str, code: Option<i32>, stderr: &str, expected: i32) {
    let branch = if code == Some(expected) {
        "expected"
    } else if code == Some(2) && skipped_device_line(stderr) {
        "skipped-device"
    } else {
        "rejected"
    };
    let _ = writeln!(std::io::stderr().lock(), "cli_ps: {label} branch={branch}");
    assert_ne!(
        branch, "rejected",
        "{label}: code {code:?}, expected {expected} or 2 with a skip line; stderr: {stderr}"
    );
}

/// `--exit-status` makes "nothing listed" exit `1`, as `pgrep` does;
/// without it the same listing exits `0`. No process has PID `u32::MAX`.
/// On a host with no device to try (`device_count()` fails: the ubuntu and
/// windows runners) that still holds. On a host whose GPU source fails (a
/// sandbox that refuses `proc_listpids`, possibly a macos-latest VM) both
/// listings exit `2` with a skipped-device line instead, since nothing
/// could be listed; `accept` takes that branch only with the line.
#[test]
fn ps_exit_status_is_1_when_nothing_is_listed_and_opt_in() {
    let (code, stderr) = hmn(&["ps", "--pid", "4294967295", "--exit-status"]);
    accept("with --exit-status", code, &stderr, 1);
    let (code, stderr) = hmn(&["ps", "--pid", "4294967295"]);
    accept("without --exit-status", code, &stderr, 0);
}

/// Run `hmn` with `args` under the field report's profile P, which denies
/// `process-info*` except on itself, returning its exit code, stdout and
/// stderr; `None` when `sandbox-exec` cannot apply a profile because this
/// test already runs inside a sandbox.
#[cfg(target_os = "macos")]
#[allow(clippy::expect_used)] // test-only
fn hmn_under_denied_process_info(args: &[&str]) -> Option<(Option<i32>, String, String)> {
    let out = Command::new("/usr/bin/sandbox-exec")
        .arg("-p")
        .arg("(version 1)(allow default)(deny process-info*)(allow process-info* (target self))")
        .arg(env!("CARGO_BIN_EXE_hmn"))
        .args(args)
        .output()
        .expect("failed to run sandbox-exec");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    if stderr.contains("sandbox_apply") {
        let _ = writeln!(
            std::io::stderr().lock(),
            "cli_ps: skipped, already sandboxed: {stderr}"
        );
        return None;
    }
    Some((
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr,
    ))
}

/// Inside a sandbox that denies `process-info*`, `hmn ps` cannot list its
/// one device: it names the device with a skip line, prints nothing on
/// stdout, closes with the all-failed line and exits `2`, where it printed
/// an empty table, `0 GPU processes found.` and exit `0`. With
/// `--exit-status`, the empty listing exits `2` (can't tell), not `1`
/// (nothing matched).
#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires a usable Metal device and an unsandboxed parent (it applies a Seatbelt profile with /usr/bin/sandbox-exec)"]
fn ps_exits_2_with_the_skip_line_when_process_info_is_denied() {
    let Some((code, stdout, stderr)) = hmn_under_denied_process_info(&["ps"]) else {
        return;
    };
    assert_eq!(code, Some(2), "stderr: {stderr}");
    assert!(skipped_device_line(&stderr), "stderr: {stderr}");
    assert!(
        stderr
            .lines()
            .any(|l| l == "hmn: ps: no device could be queried, so nothing could be listed"),
        "stderr: {stderr}"
    );
    assert!(stdout.is_empty(), "{stdout:?}");
    assert!(
        !stderr.contains("0 GPU processes found"),
        "stderr: {stderr}"
    );

    let Some((code, _stdout, stderr)) =
        hmn_under_denied_process_info(&["ps", "--pid", "4294967295", "--exit-status"])
    else {
        return;
    };
    assert_eq!(code, Some(2), "stderr: {stderr}");
}
