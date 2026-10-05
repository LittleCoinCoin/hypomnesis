# PR B review record

How PR B's changes were reviewed before it was marked ready, 2026-10-03 to 2026-10-05. Each review was an AI agent (Claude Sonnet, or Claude Opus for the test reviews), run read-only in a separate session with its own scratch worktree. Each one re-ran the gates itself rather than trusting the implementer's report, and tried wrong implementations to see whether a test caught them. Findings were adjudicated by the coordinating session and, where they changed scope, by the contributor.

## Adversarial review of the new `unsafe` (item 2)

Requested in the #6 review: "Item 2 … brings the `sysctl` + `kinfo_proc` parser, which is new `unsafe` code, so it gets that review in PR B."

- **`sysctl` declaration** (`src/gpu/metal.rs`, `libsystem_ffi`): `name: *mut c_int, namelen: u32, oldp, oldlenp: *mut usize, newp, newlen: usize` matches `<sys/sysctl.h>`.
- **The call's `// SAFETY:`** holds. The MIB is `[i32; 4]` with `namelen` 4. The buffer is `[u8; 648]`, alignment 1, valid for `len` bytes, and `len` is in/out; the kernel never writes past the length it was given. `newp`/`newlen` are null/0.
- **errno**: `last_os_error()` is the first call after the `unsafe` block. Only `proc_pidpath`'s errno reaches the decision (`Refused { .. }` ignores the `sysctl` errno), so capture order cannot change an answer.
- **Parser bounds** (`src/gpu/kinfo.rs`): `as_chunks` and `.get(..)` only, no indexing, no `as`. A partial record is rejected. `rc` is checked before `len`, so a refused call that leaves `len == 648` over a zeroed buffer is never parsed as a PID 0 record.
- **Mutations**, each run against the suite. Every one failed at least one test:

  | Wrong implementation | Caught by |
  |---|---|
  | `pid == 0` special case | `macos_smoke::process_exists_under_sandbox_profiles` (profile Q row 0); hardware-only, `#[ignore]` |
  | `rc` ignored | `classify_kern_proc_pid_refused_call_is_not_a_record`, the sandbox table |
  | refused → always `Some(false)` / always `None` | `decide_exists_follows_the_lookup_rule`, the sandbox table |
  | whole-record check removed | `parse_kinfo_records_rejects_a_partial_record` |
  | `p_comm` at 244 or `p_pid` at 44, consistently in constant and fixture | `metal::tests::kern_proc_pid_record_matches_the_kernel_for_this_process` (live read; arm64 in CI, x86_64 under Rosetta) |
  | `ESRCH` rule inverted | `decide_exists_follows_the_lookup_rule`, the sandbox table |
  | record PID not compared with the requested one | `classify_kern_proc_pid_record_must_name_the_requested_pid` |
  | `sysctl` asked although a path was found | `decide_exists_follows_the_lookup_rule` (a `Cell` flag) |

- **Found and fixed:** a zombie (exited, not yet reaped) reads `Some(true)`, because `KERN_PROC_PID` keeps its record until the parent reaps it. Linux behaves the same way. The `process_exists` rustdoc and the CHANGELOG say so.

## Per-item reviews

| Item | Review | Verdict | Notes |
|---|---|---|---|
| 1 `bounds_check` | implementer's gates (mechanical) | — | |
| 2 `process_exists` | Sonnet, adversarial | PASS | above |
| 5 skipped devices, exit 2 | Sonnet, adversarial | PASS | confirmed that both `cli_ps` tests accept exit 2 only with a `(skipped)` line, as asked in #6 |
| 6 remedy text | Sonnet, adversarial | PASS | 8 mutations; the `hmn watch` growth-hint call site is pinned by grep only |
| 7 `n/a` cells | implementer's gates (mechanical) | — | |
| 8 docs, harness, field check | Sonnet, adversarial, twice | PASS WITH NOTES | see below |

## Review of the added tests

Two reviewers, one for the library tests and one for the `hmn` tests, asked three questions of each new test: can it fail for a plausible wrong implementation; does it read like the crate's existing tests; does it follow CONVENTIONS.md's *Test-Module Lint Allowances*. They ran mutations and swapped every added `#[allow]` for `#[expect]` under stable default, stable `--all-features`, `x86_64-unknown-linux-gnu` and 1.88. Fixes:

- `process_exists_under_sandbox_profiles` passed without exercising profiles P and Q when it ran inside a sandbox, or with `HMN_PE_CHILD` inherited. It fails in both cases.
- The macOS `NoGpuSource` text was checked only for `Metal`. It is pinned byte for byte.
- No unsandboxed test saw an off-by-one in the Metal arm of `bounds_check`. A unit test pins both edges.
- No test pinned the all-failed closing line. `ps_exits_2_with_the_skip_line_when_process_info_is_denied` asserts it.
- `clippy::expect_used` on the `metal.rs` test module suppressed nothing, so it was dropped.
- Test expectations use `#[cfg]` constants rather than `cfg!`. Ignore reasons start with "requires". `#[cfg]` comes before `#[test]`.
- Removed: a test that restated `REMEDY_OUTSIDE_SANDBOX`'s definition (the `#[cfg]` twins of `format_ps_summary` pin it), two alignment tests that the existing `Table` tests already covered, and two duplicate assertions.

Pinned by grep rather than a run, because a run needs hardware or stderr this machine cannot produce: `run_ps` passing the failed-device count to `ps_exit_code` (a partial failure needs two GPUs), and `hmn watch` passing the platform flag to its growth hint.

## Item 8 review, and the revision it led to

First review: PASS WITH NOTES. Adopted:

- `ps_json` captures keep only the key names: a row count is live data and differs between runs.
- `harness/codex.sb`, a copy of an Apache-2.0 file from openai/codex, gets its licence and NOTICE attribution in `harness/CODEX_PIN`.
- The harness's capture and compare scripts are written in Python (stdlib only), so they read and run like the field-check probes. `sandbox.sh` stays a shell wrapper around `/usr/bin/sandbox-exec`.
- The macOS name sentence and the README/`--help` Security note were reworded so they no longer contradict the rustdoc.

Not adopted: rows that vanish from a capture (PR C's guards require at least one compared row per policy), and committing the App Sandbox proof commands (the control is already on record).

Second review, of the revision: PASS WITH NOTES. The Python scripts match the shell ones on 31 argument and capture combinations: same stdout and exit codes, and one extra usage line on stderr for a missing directory. Adopted: three sentences that described a release rather than the tool (the `ledger`-only case in README item 9, the CHANGELOG bullet, the Intel row of ROADMAP) and the README sentence "only a sandbox withholds a macOS name" are reworded as plain statements.

Recorded and left as is: the captures' process names are unsalted 8-hex SHA-256 prefixes, so a common app name can be recovered by trying candidates.
