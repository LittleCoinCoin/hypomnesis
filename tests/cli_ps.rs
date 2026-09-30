// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end exit-code tests for `hmn ps` that hold on any machine, GPU
//! or not — so, unlike the `live_*` tests, they are not `#[ignore]`d.
//! `#![cfg(feature = "cli")]`: they run the compiled binary through
//! `env!("CARGO_BIN_EXE_hmn")`, which Cargo only defines when the `hmn`
//! target (`required-features = ["cli"]`) is built.

#![cfg(feature = "cli")]

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

/// `--exit-status` makes "nothing listed" exit `1`, as `pgrep` does;
/// without it the same listing exits `0`. No process has PID `u32::MAX`,
/// and on a machine with no GPU source nothing is listed at all.
#[test]
fn ps_exit_status_is_1_when_nothing_is_listed_and_opt_in() {
    let (code, stderr) = hmn(&["ps", "--pid", "4294967295", "--exit-status"]);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    let (code, stderr) = hmn(&["ps", "--pid", "4294967295"]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
}
