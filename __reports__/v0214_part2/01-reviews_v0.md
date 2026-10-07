# PR C review record

This record covers how PR C's changes were reviewed before the PR was opened, on 2026-10-07. Each review was an AI agent, run read-only in its own scratch worktree. The reviewers were:
- a Claude Sonnet spec verifier for each item;
- a Claude Opus reviewer for the tests of each item;
- an independent Claude Opus reviewer for the new FFI, with no part in writing the code or its spec.

Every reviewer re-ran the gates itself instead of trusting the implementer's report, applied wrong implementations to see which tests caught them, and reverted each one. The coordinating session adjudicated the findings. Each adopted finding went back to the implementer, who folded the fix into the commit that owns the file. A further verifier then re-checked that change.

## Independent review of the new FFI (item 3)

The maintainer had PR B's `sysctl`/`kinfo_proc` code reviewed "by an agent with no part in the earlier work". The new code is the `KERN_PROC_ALL` call in `src/gpu/metal.rs`, its pure classifier in `src/gpu/kinfo.rs`, and the errno handling of `proc_listpids`, `ledger` and `proc_pidpath`.

- **Declaration and constants.** `sysctl(name, namelen: u32, oldp, oldlenp: *mut usize, newp, newlen)` matches `<sys/sysctl.h>` (BSD). `CTL_KERN` 1, `KERN_PROC` 14, `KERN_PROC_ALL` 0, `EPERM` 1, `ESRCH` 3, `ENOMEM` 12, `PROC_ALL_PIDS` 1 and `PROC_PIDPATHINFO_MAXSIZE` 4096 match the SDK headers. `namelen` is `mib.len()`.
- **The call, against XNU (`bsd/kern/kern_sysctl.c` `sysctl_prochandle`, `sysdoproc_callback`; `kern_newsysctl.c` `userland_sysctl`).**
  - The size probe returns every record plus `KERN_PROCSLOP` (5 records); measured, 1059 probed against 1054 filled.
  - On a short buffer the kernel copies the whole records that fit, fails with `ENOMEM` and writes back a `len` of 0. Measured with 3- and 10-record buffers.
  - `EPERM` leaves `len` and the buffer untouched.
  - The fill buffer is zeroed and sized to whole 648-byte records with ⅛ slack.
  - `rc` is read first, and on failure neither `buf` nor `len` is read.
  - `ENOMEM` is retried, at most four calls in all.
  - `errno` is read before any allocation or `drop`.
  - Every `// SAFETY:` holds. No UB, no read of uninitialised memory, no unchecked size arithmetic.
- **Parser reuse.** `kinfo::parse_kinfo_records` is the only parser; there is no second layout.
- **The record-size guard.** `trust_kinfo_listing` trusts a listing only when `KERN_PROC_PID` for the caller is exactly one 648-byte record. With a probe-sized buffer, a kernel whose `kinfo_proc` is larger than 648 bytes never answers `ENOMEM` to `KERN_PROC_ALL`, and its records would pass the whole-records check whenever their total is a multiple of 648. The self lookup goes through the same handler, so a larger record reads `ENOMEM` → `Unusable`, and a smaller one a partial record → `Unusable`.
- **errno, measured with an independent C probe under eight Seatbelt profiles:**

  | Call | unsandboxed | report's profile (P), S, S0 | `kern.proc` denied (Q) | pidinfo denied (D) | ledger denied (L) | Codex (C) |
  |---|---|---|---|---|---|---|
  | `proc_listpids` | ok | 0, `EPERM` | 0, `EPERM` | ok | ok | ok |
  | `KERN_PROC_ALL` | ok | ok | -1, `EPERM` | ok | ok | -1, `EPERM` |
  | `proc_pidpath` other / missing | ok / `ESRCH` | `EPERM` / `ESRCH` | as P | as P | ok / `ESRCH` | ok / `ESRCH` |
  | `ledger` self / other / missing | ok / ok / `ESRCH` | ok / `EPERM` / `ESRCH` | as P | ok / ok / `ESRCH` | `EPERM` / `EPERM` / `ESRCH` | ok / ok / `ESRCH` |

  XNU looks the PID up (`proc_find`) before the sandbox check, so a missing PID reads `ESRCH` even inside a sandbox. Only `EPERM` is ever counted as denied or refused.
- **Mutations.** Each mutation below fails a test:

  | Wrong implementation | Caught by |
  |---|---|
  | `namelen` 2, or `KERN_PROC_PID` in the `KERN_PROC_ALL` MIB | `list_kinfo_all_holds_this_process` (live; arm64 and x86_64 under Rosetta) |
  | the partial `ENOMEM` fill parsed; `rc` ignored | `classify_kinfo_all_enomem_is_retry_even_when_the_buffer_holds_records` |
  | `len` not clamped to the buffer | `classify_kinfo_all_an_unusable_success_is_failed` |
  | `EPERM` and `ESRCH` swapped where names fall back to `p_comm`; `EPERM` defined as 2 | `name_after_pidpath_uses_comm_only_when_eperm` (literal errnos) |
  | `EPERM` and `ESRCH` swapped in `footprint_from_errno` or `libproc_outcome` | `footprint_from_errno_only_eperm_is_denied`, `libproc_outcome_only_eperm_is_refused` |
  | the size guard trusting any self lookup | `trust_kinfo_listing_keeps_records_only_when_self_is_one_648_byte_record` |
  | the caller, or a gone PID, counted as denied | `tally_reads_*` |

- **Residuals, checked by reading and review only.** The glue between the `KERN_PROC_ALL` FFI call and its classifier has no test. That covers the `rc`, `errno` and `len` passed across, the slack, the probe's `rc` check and the bound of four attempts. The same holds for:
  - the libproc-first order in `list_pids`;
  - the phase-2 `errno` read in `list_libproc_pids`;
  - an unresolved ledger index read as entry 0;
  - `kern_proc_pid_comm` going through `classify_kern_proc_pid`, whose omission changes no output (a refused call's zeroed buffer reads as an empty name);
  - a refused and a failed `KERN_PROC_ALL`, which both end as "can't enumerate".

  An absurd kernel probe size is handled only by `vec!`'s own allocation failure. If libproc returned 0 without setting `errno`, the worst outcome would be a `p_comm` name, never a denial.

## Per-item reviews

| Item | Review | Verdict | Notes |
|---|---|---|---|
| 3 `KERN_PROC_ALL`, four-outcome read, `p_comm` | Sonnet spec verifier; Opus test review; Opus FFI review | PASS WITH NOTES (each) | 11 should-fix across the three, all adopted; a delta verifier then passed the revision |
| 4 `gpu_process_listing`, `ProcessListDenied` | Sonnet spec verifier; Opus test review | PASS WITH NOTES | public API additive only (rustdoc JSON compared); 5 should-fix adopted; delta verifier passed |
| 6 `unreadable` counts, exit status, `watch` notices | Sonnet spec verifier; Opus test review | PASS WITH NOTES | seam exact (`1058 unreadable` and the probe's `denied=1058` in 4 of 4 runs); 3 should-fix adopted |

## Review of the added tests

Each reviewer asked three questions of every new test:
1. Can it fail for a plausible wrong implementation?
2. Is it tautological?
3. Does it follow the crate's idiom and CONVENTIONS.md's *Test-Module Lint Allowances*?

To answer them, the reviewers ran about 190 mutants and swapped every added `#[allow]` for `#[expect]` under stable default, stable `--all-features`, `x86_64-unknown-linux-gnu` and 1.88. The reviews led to these fixes, which the final tree keeps:

- **Linux and Windows.** An integration test pinned `denied_pids` empty there, and it passes vacuously on a runner with no GPU. A unit test of `listing_without_denials`, under its own `cfg`, pins it instead.
- **Strings.** `ProcessListDenied`'s `Display` was checked against a list of forbidden words, and `; run as root` passed. It is now pinned byte for byte, as `NoGpuSource` is.
- **The `failure_detail` test.** It adds the remedy to a denial and to nothing else, including `NoGpuSource`, which is what a macos-latest VM gives.
- **Harness.** `compare_names.py` and `count_denied.py` passed on empty input. They exit 1 when there is nothing to compare, or when the listing lacks the probe itself.
- **Sandbox test.** `gpu_process_listing_under_sandbox_profiles` fails rather than passes when it runs inside a sandbox or with its child variable inherited, as PR B's `process_exists_under_sandbox_profiles` does.

## Slimming before the PR opened

The first implementation was correct but carried seams that existed to make each review survivor testable: closure injection, a retry driver, a test-only enum, helpers for single messages. It also carried tests that pinned those seams rather than a behaviour. Three read-only audits, one per item, compared every addition with the maintainer's comment on #3, R01's design and his own code. The additions were then cut back to R01's shape.

- **What stayed.** The pure classifiers that R01's decisions hinge on stay: `classify_kinfo_all` (the shape of PR B's `classify_kern_proc_pid`), `footprint_from_errno`, `libproc_outcome`, `name_after_pidpath`, `tally_reads`, `decide_listing` and `remedy_clause`. Each has one or two literal tests.
- **The record-size guard stayed** on the contributor's decision. A `kinfo_proc` of another size passes the whole-records check once in 81, which would give a silent wrong listing.
- **What went.** The removed items are:
  - `fill_kinfo_all_with`, `KinfoRead`, `comm_from_lookup` and `enumerate_pids_with`;
  - `process_list_denied`, now inside `decide_listing`;
  - the `compile_fail` doctest;
  - `tests/macos_sandbox.rs`, which is now a smaller test in `tests/macos_smoke.rs`;
  - the `Branch` enum and line matcher in `tests/cli_ps.rs`, now two `else if` arms in the existing `accept`;
  - the `watch.rs` message helpers.
- **The one change of behaviour.** `hmn watch` prints the "N unreadable … they are not followed" line for `--follow-new` only, as R01 states. The first implementation also printed it for plain auto-select.
- **Equivalence.** The slim tree was checked against the first one in one boot. That covered 22 commands under eight Seatbelt profiles, 528 cells of exit code, stdout and stderr, plus `compare.py` and a rustdoc-JSON diff of the 486 crate items. There were no differences except that line and its `--help` sentence.

Recorded and left as is, each stated in the PR body:
- The Metal arm's wiring into `decide_listing` is caught only by the `#[ignore]`d sandbox test.
- `run_watch` passing an empty denied list to `missing_pid_notices` is caught by nothing, because `process_exists` answers `Some(true)` for a denied PID.
- No test pins these:
  - `accept`'s labels;
  - the `--follow-new`-only gate of the "not followed" line;
  - the "found no GPU processes" count;
  - `GpuProcessListing` staying `#[non_exhaustive]`;
  - the count saturating at `u32::MAX`.

  The CI log, the profile-S field runs and review cover them.
