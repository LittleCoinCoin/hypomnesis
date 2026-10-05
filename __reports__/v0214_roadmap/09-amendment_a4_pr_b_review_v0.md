# Gap Analysis: v0214-sandbox — the maintainer's review of PR B (A4)

## Problem Statement
PR B (mi-for-the-rust-of-us/hypomnesis#7, head 4d32647) received CHANGES_REQUESTED from PCfVW on 2026-10-05. He ran the CI gate set on Windows and on Ubuntu WSL2 (stable and 1.88), clippy for `aarch64-apple-darwin` and Linux, `cargo deny`, the package list, the 19 `#[ignore]`d live tests, and each of the 17 commits on its own. A separate agent also reviewed the `sysctl`/`kinfo_proc` code adversarially: it found no memory-safety problem and no UB, a declaration that matches the C prototype, and tests that each fail when their fix is reverted. Two changes are requested before merge, plus six nits to take now or in PR C.

Requested:
1. **A larger `kinfo_proc` reads as "no such process".** On a kernel whose record exceeds 648 bytes, XNU copies nothing and `sysctl` fails with `ENOMEM`, `len` 0 (`sysdoproc_callback`, `sysctl_prochandle`). `classify_kern_proc_pid` maps that to `Refused { errno: 12 }`, and `decide_exists` ignores `sysctl`'s errno, so with `proc_pidpath`'s `ESRCH` the answer is `Some(false)`, for PID 0 and zombies too. `kern_proc_pid_raw`'s rustdoc and R01 promise "can't tell". Fix: `rc != 0 && errno == ENOMEM` → `Unusable`, with a named const, and two tests. `Refused` must not be narrowed to `EPERM` only.
2. **"file ownership" → "process ownership"**, at five sites (`main.rs` ×2, `lib.rs`, README capability table, `metal.rs` module doc).

Nits: a `remedy_text` purpose enum instead of the `"for names"` string; `namelen` derived instead of a bare `4`; `sysctl`/`sysctlbyname` are BSD, not POSIX; `comm_of`'s `// BORROW:` and `P_COMM_SIZE - 1`; the `cli_ps` sandbox test should fail, not pass, when already sandboxed; a pre-existing broken intra-doc link to `super::dxgi::DxgiQueryResult`.

## Evidence
The review text, and the sites re-read at 4d32647: `kinfo.rs:156` (`rc != 0` → `Refused`), `:199` (`Refused` → `(path_errno == ESRCH).then_some(false)`), `metal.rs:774-775` ("classifies as `Refused` ("can't tell")"), R01 ~150-151 ("the whole-records length check stays the parser's only guard"), and the five "file ownership" sites.

## Root Cause
The leaf `process_exists_kinfo` specified `rc != 0` → `Refused` for every errno and assumed `Refused` meant "can't tell". `decide_exists` reads only `proc_pidpath`'s errno, so a failed `sysctl` with any errno behind an `ESRCH` path reads `Some(false)`. The "file ownership" wording came from part1_close's pointer sentence.

## Impact Assessment
- Scope: `src/gpu/kinfo.rs`, `src/gpu/metal.rs`, `src/gpu/mod.rs`, `docs/roadmap-v0.2.14.md`, `src/bin/hmn/{format,ps,watch,main}.rs`, `src/lib.rs`, `README.md`, `tests/cli_ps.rs`. Behaviour change: only `ENOMEM` from `KERN_PROC_PID`, which moves from `Some(false)` to `None`. Output text is unchanged everywhere.
- PR C contracts: `remedy_text(outside_sandbox, "for names")` becomes `remedy_text(outside_sandbox, RemedyPurpose::Names)`. `ps_watch_unreadable.md`, which extends `remedy_text` and cites the seam test, is updated in the same roadmap commit. `kinfo_enumeration` inherits `ENOMEM` as a named const beside `ESRCH`.
- History: new commits on top of 4d32647 (the user's choice), with no force-push on a branch the maintainer reviewed commit by commit.

## Proposed Solution
One leaf, `part1/maintainer_review_b.md`, five steps and five commits: the `ENOMEM` fix with its tests and docs; the wording; `RemedyPurpose`; the `metal.rs` nits; the `cli_ps` fail-not-skip. The enum follows the house style of `ps::SortKey` and `ps::PsJudgement` (domain-named, `Debug, Clone, Copy, PartialEq, Eq`, the "Binary-internal dispatch enum … matched exhaustively by …" doc formula, a `///` on every variant, one exhaustive `match`). It is named `RemedyPurpose` rather than the review's illustrative `Purpose`, because the existing enums are named after their domain. An adversarial Sonnet verifier checks Steps 1 and 3 before the push. The PR reply answers each point, including that naming note, which the user asked for.

## Recommendations
1. Approved by the user on 2026-10-06: all nits including the enum, named `RemedyPurpose`; new commits on top.
2. After the push, reply on the PR point by point and re-request review, with the user's yes.
