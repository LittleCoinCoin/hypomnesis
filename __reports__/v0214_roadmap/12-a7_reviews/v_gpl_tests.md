# Review of the tests added by `gpu_process_listing` (task/gpu_process_listing @ c456939, on 8519f1a)

**Verdict: PASS WITH NOTES** (0 blocker, 4 should-fix, 6 note)

Every new or widened test failed under at least one mutation of the code it guards (table below). The red commit 38c151b gives `1 passed; 8 failed` in the lib and a failing sandbox test, with 9 `panicked at` and 0 `error[E`; the one pass is the signature guard, as the leaf says. The sandbox test held up against every "profile silently not applied" attack I could build. The gaps are elsewhere:

- On Linux and Windows nothing pins `denied_pids` empty: a `listing_without_denials` that reports a denial survives every test on every target.
- The unsandboxed row never checks `denied=0`, so a spurious denied PID on macOS survives the whole suite, the `#[ignore]`d sandbox test included.
- The Display test's word list lets "outside", "as root" and other remedies through.
- The saturation test cannot tell saturation from truncation.

As in the kinfo review, several of these gaps come from the leaf's test list rather than from the implementer.

Method: a detached worktree at c456939 (`scratchpad/v_gpl_tests`) and a second one for the lint swap (`scratchpad/v_gpl_tests_lint`), both removed at the end. Each mutation was one edit, then the command named in the table, then `git checkout -- src tests` (`git status --short` was empty after each one). The script is `scratchpad/vgt_mut.py` and its output is `scratchpad/vgt_mut.out`. The sandbox mutants ran `cargo test --locked --all-features --test macos_sandbox -- --ignored --nocapture` unsandboxed (probe rc 0, so this host is not inside a sandbox).

## Findings, most severe first

### 1. should-fix: on Linux and Windows, nothing pins `denied_pids` empty; `gpu_process_listing_has_no_denied_pids_off_macos` passes vacuously on a runner with no GPU
- `tests/smoke.rs:219-231`, guarding `listing_without_denials` at `src/gpu/mod.rs:553-563` and the three non-Metal arms at `:468, :491, :509`.
- Evidence:
  - On a Linux runner with no NVIDIA driver, `nvml::list_compute_processes` and `nvidia_smi::query_compute_apps` both give `None`, and `bounds_check` gives `Ok(())` (no Metal, NVML or DXGI count, `src/gpu/mod.rs:721-750`). So the listing is `Err(NoGpuSource)`, and the test only checks that the error is not `ProcessListDenied`. Its `Ok` arm never runs.
  - Windows depends on whether PDH answers on a hosted runner. I could not run Windows, so that case is unverified. Docker is installed here, but its daemon is not running.
  - Mutant W6′ (`listing_without_denials` made non-`const`, returning `denied_pids: vec![1]`): `cargo test --locked --all-features` gives 145 + 248 + … passed, 0 failed. `cargo clippy --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings` exits 0. On macOS the fn is compiled through `nvidia-smi-fallback` and still never called by a test.
  - Mutant W7 (the nvidia-smi arm returns `Err(ProcessListDenied { denied: 1 })` on an empty list): all tests pass, and the Linux check exits 0.
- The leaf's static gate `grep -c 'listing_without_denials(' src/gpu/mod.rs` = 4 (measured: 4) counts call sites only. It holds under W6′ and under W7. "`denied_pids` is empty by construction" therefore rests on the body of a fn that no test reads.
- Fix:
  - Add a unit test under the helper's own `cfg(any(all(linux, nvml), all(windows, pdh), nvidia-smi-fallback))`. It then runs in the `Tests` step of every CI job, GPU or not:

    ```rust
    fn listing_without_denials_denies_nothing() {
        let listing = listing_without_denials(vec![row(7)]);
        assert!(listing.denied_pids.is_empty(), "{:?}", listing.denied_pids);
        assert_eq!(listing.entries.len(), 1);
    }
    ```

    A prototype passed unmutated, caught W6′ (`listing_without_denials_denies_nothing ... FAILED`), and left clippy clean on stable and on the Linux target.
  - For W7, add a static gate: `ProcessListDenied` is constructed only in `decide_listing`, so `grep -c 'Err(HypomnesisError::ProcessListDenied' src/gpu/mod.rs` must be 1 outside `mod tests`.

### 2. should-fix: the sandbox test's unsandboxed row accepts a spurious denial; a fake denied PID survives the whole suite
- `tests/macos_sandbox.rs:153`: `Expect::Open` checks only `ok`, `caller_denied=false` and `overlap=false`.
- Evidence: mutant W5, a Metal arm that pushes PID 1 into `denied_pids` before `decide_listing`. The lib gives 77 passed, `macos_smoke` 6 passed, and `macos_sandbox -- --ignored` 1 passed.
  - S stays `Partial`, and S0, P and L stay `Denied` with the count one higher.
  - Unsandboxed, the line reads `LISTING ok denied=1 …` and is accepted.
  - `gpu_process_listing_never_lists_the_caller_as_denied` also passes, because PID 1 is neither the caller nor a row.

  `hmn ps` on an unsandboxed Mac would then print a refusal count that is not real.
- Measured unsandboxed truth: `denied=0` (my baseline, and the implementer's `gpl14.log`).
- Fix: `Expect::Open` also requires `denied == Some(0)`. A prototype passed unmutated and caught W5 (`unsandboxed: LISTING ok denied=1 entries=24 …`, FAILED). Optionally, also have `macos_smoke::gpu_process_listing_never_lists_the_caller_as_denied` assert `denied_pids.is_empty()` when `sandbox_can_apply_a_profile()` is true; the helper already exists in that file. That would put the check in a test that runs by default.

### 3. should-fix: the Display test's forbidden-word list misses "outside", "root" and other remedies
- `src/error.rs:151-158`: the list is `["re-run", "sandbox", "elevated", "sudo"]` (the leaf's list, so this is a spec defect).
- Evidence: mutants that append a suffix to the `#[error]` text. Caught: X1 `; re-run outside the sandbox`, X5 `; needs elevated privileges`, X6 `; try sudo`, X11 count dropped, X12 `refused` dropped. **Survived:**

  | Suffix appended to the `#[error]` text | Result |
  |---|---|
  | X2 `; run it outside` | 77 passed |
  | X3 `; run as root` | 77 passed |
  | X4 `; grant Full Disk Access` | 77 passed |
  | X7 `; ask an administrator` | 77 passed |
  | X8 `; Sandbox refused it` (case-sensitive match) | 77 passed |
  | X9 `; rerun hmn from a terminal` | 77 passed |
  | X10 `{denied}0 refused` (prints `9080`; `contains("908")` holds) | 77 passed |

  A word list can always be dodged. PR B's review had the same problem with `NoGpuSource` (it was checked only for `Metal`) and fixed it by pinning the text byte for byte, which `no_gpu_source_display_names_the_backends_of_the_platform` does in the same module.
- Fix: `assert_eq!(text, "process list unreadable: 908 refused, none other than the caller's could be read")`. The word loop can stay as documentation of intent. A prototype caught X2, and it catches every row above by construction.

### 4. should-fix: `process_list_denied_saturates_the_count` cannot tell saturation from truncation
- `src/gpu/mod.rs:853-856` uses `usize::MAX`, and on a 64-bit target `usize::MAX as u32 == u32::MAX`.
- Evidence: mutant P4 (`denied as u32`, with `#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]` as a `// CAST:` site would carry) gives 77 passed. The truncating cast is the very bug the test is named after.
- Fix: test `process_list_denied(0, 4_294_967_296)` → `Some(u32::MAX)`; a truncating cast gives `Some(0)` there. Replace the `usize::MAX` case or keep both. A prototype caught P4.

### 5. note: the signature guard adds nothing over the integration tests, and a generic signature still coerces
- `src/gpu/mod.rs:892-897`.
- Evidence:
  - G1 (`gpu_processes` returns `GpuProcessListing`) is caught by the guard (E0308), and equally by `tests/smoke.rs` (E0599/E0277 on `rows.iter()`).
  - G2 (`impl Into<u32>`), G3 (`<I: Into<u32>>`) and G4 (`gpu_process_listing(impl Into<u32>)`) coerce to the fn pointer, so the guard **passes**. They are caught only because `smoke`, `macos_smoke`, `live_gpu` and `macos_sandbox` call `gpu_processes(0)` / `gpu_process_listing(0)` with a literal (E0277 `u32: From<i32>`).
  - The guard sits inside the crate, so it cannot see visibility or the `lib.rs` re-export either. Those break the lib or the integration tests anyway.
  - An `impl TryInto<u32>` parameter would coerce and still accept `gpu_processes(0)`. That is the remaining residual, and it is benign in practice.
- Fix: move the coercion into `tests/smoke.rs` with the public paths: `let _: fn(u32) -> hypomnesis::Result<Vec<hypomnesis::GpuProcessEntry>> = hypomnesis::gpu_processes;`. Name the literal-call tests as what catches a generic parameter. Or record the guard as return-type-only.

### 6. note: the sandbox test cannot see "the caller listed as denied", though gate 15 credits it with that
- Evidence: mutant B4 (`tally_reads` keeps the caller in `denied`). `macos_sandbox -- --ignored` gives 1 passed, and `macos_smoke` gives 6 passed.
  - The caller's own read is refused only under L, and `Expect::Denied` sees just a count.
  - Under S and unsandboxed, the caller reads itself.
  - B4 is caught only by the kinfo leaf's unit test `tally_reads_never_lists_the_caller_as_denied`.
- Fix: correct gate 15's "catches" text. The unit test is the guard.

### 7. note: `decide_listing_sorts_by_pid_and_keeps_denied_pids` uses data that breaks the invariant the macOS tests assert
- `src/gpu/mod.rs:876`: `decide_listing(vec![row(9), row(3), row(5)], vec![5, 7], 1)`, where PID 5 is both a row and a refusal. `macos_smoke.rs:327` and the sandbox `overlap` field assert that this never happens.
- Evidence: mutant D8 (a decision that drops denied PIDs that are rows, arguably a hardening) fails this test at `:882`.
- Fix: use `vec![6, 7]`, so the test does not pin an impossible state.

### 8. note: the widened `macos_smoke` test's comment is stale, and the module doc overruns its wrap
- `tests/macos_smoke.rs:126-127` still lists outcome "(c) Err(NoGpuSource) on a non-GPU host". The arm now also accepts `ProcessListDenied` (a sandboxed run), and `tests/smoke.rs` updated its own comment for the same change (OWNERSHIP).
- `tests/macos_smoke.rs:5` is 92 columns in a `//!` block wrapped at about 72.
- Fix: add "(d) Err(ProcessListDenied) inside a sandbox that refuses every other process", and re-wrap line 5.

### 9. note: the red commit is still a separate commit
- 38c151b fails `cargo test` (9 `panicked at`, 0 `error[E`, measured). SQUASH SCOPE requires folding it into da3d698 before the PR branch is pushed.

### 10. note: on stable, the `compile_fail,E0639` doctest passes for any compile error
- Evidence: N3 misspells the field (`denied_pid`, E0560), and both doctests pass, so stable does not check the code. N1, which removes `#[non_exhaustive]`, **is** caught on stable: the literal then compiles, and the `compile_fail` doctest fails.
- The other wrong-reason cases are covered elsewhere: the positive doctest and the integration tests catch a missing re-export or a renamed field.
- Fix: none needed. The brief's question is answered: yes, it catches a struct that is not `#[non_exhaustive]`.

## Mutation table

| Mutation | Test that caught it, or "survived" |
|---|---|
| P1 `process_list_denied`: `others_read <= 1` (the caller counted as read) | process_list_denied_is_none_when_another_process_was_read, decide_listing_sorts_by_pid_and_keeps_denied_pids |
| P2 ignores `others_read` | same two |
| P3 ignores `denied` | process_list_denied_is_none_when_nothing_was_denied, decide_listing_with_nothing_denied_is_ok_even_when_empty |
| P4 truncating `denied as u32` | **survived** (finding 4); caught by the prototype `…_past_u32_max` |
| P5 saturates to 0 | process_list_denied_saturates_the_count |
| P6 the stub's `Some(0)` | 6 of the 7 decision tests |
| D1 decide counts `entries` as read | decide_listing_is_denied_when_only_the_callers_own_row_was_read |
| D2 decide never denies | decide_listing_is_denied_when_only_the_callers_own_row_was_read |
| D3 unsorted | decide_listing_sorts_by_pid_and_keeps_denied_pids |
| D4 sorted descending | decide_listing_sorts_by_pid_and_keeps_denied_pids |
| D5 drops `denied_pids` | decide_listing_sorts_by_pid_and_keeps_denied_pids |
| D6 counts `entries` as denied | decide_listing_is_denied_when_only_the_callers_own_row_was_read |
| D7 `Err` on an empty list with nothing denied | decide_listing_with_nothing_denied_is_ok_even_when_empty |
| D8 drops denied PIDs that are rows | decide_listing_sorts_by_pid_and_keeps_denied_pids (finding 7: the test data is impossible) |
| D9 count off by one | decide_listing_is_denied_when_only_the_callers_own_row_was_read |
| D10 the caller's lone row returned as `Ok` | decide_listing_is_denied_when_only_the_callers_own_row_was_read |
| X1 `; re-run outside the sandbox` | process_list_denied_display_states_the_count_and_no_remedy |
| X2 `; run it outside` | **survived** (finding 3) |
| X3 `; run as root` | **survived** |
| X4 `; grant Full Disk Access` | **survived** |
| X5 `; needs elevated privileges` | process_list_denied_display_… |
| X6 `; try sudo` | process_list_denied_display_… |
| X7 `; ask an administrator` | **survived** |
| X8 `; Sandbox refused it` | **survived** |
| X9 `; rerun hmn from a terminal` | **survived** |
| X10 count printed as `9080` | **survived** |
| X11 count dropped | process_list_denied_display_… |
| X12 `refused` → `blocked` | process_list_denied_display_… |
| G1 `gpu_processes` returns the listing | gpu_processes_keeps_its_signature_… (E0308); also the smoke target |
| G2 `gpu_processes(impl Into<u32>)` | guard **survived**; smoke, macos_smoke, live_gpu (E0277) |
| G3 `gpu_processes<I: Into<u32>>` | guard **survived**; smoke, macos_smoke (E0277) |
| G4 `gpu_process_listing(impl Into<u32>)` | guard **survived**; macos_smoke, macos_sandbox (E0277) |
| N1 `GpuProcessListing` not `#[non_exhaustive]` | doctest `snapshot::GpuProcessListing (line 235) - compile fail` (stable) |
| N3 the doctest's field misspelt (E0560 instead of E0639) | **passes** on stable (finding 10; a test mutant, not a code mutant) |
| W1 macos_smoke rejects `ProcessListDenied` (run under `sandbox.sh P`) | gpu_processes_returns_metal_rows_for_self |
| W2 smoke rejects `ProcessListDenied` (run under `sandbox.sh P`) | gpu_processes_returns_result_or_no_gpu_source |
| W3 decide lists the caller as denied | gpu_process_listing_never_lists_the_caller_as_denied |
| W4 Metal arm puts every row's PID in `denied_pids` | gpu_process_listing_never_lists_the_caller_as_denied |
| W5 Metal arm adds a spurious denied PID 1 | **survived** lib, macos_smoke and macos_sandbox (finding 2); caught by the prototype `denied == Some(0)` |
| W6′ `listing_without_denials` returns `denied_pids: vec![1]` | **survived** all tests, macOS and the Linux clippy target (finding 1); caught by the prototype `listing_without_denials_denies_nothing` |
| W7 nvidia-smi arm returns `ProcessListDenied` on an empty list | **survived** all tests (finding 1) |
| B1 Metal arm skips the decision | gpu_process_listing_under_sandbox_profiles (S0) only; lib and macos_smoke pass |
| B2 Metal arm passes `others_read` 0 | gpu_process_listing_under_sandbox_profiles (S) only |
| B3 Metal arm drops `denied_pids` | gpu_process_listing_under_sandbox_profiles (S) only |
| B4 `tally_reads` keeps the caller in `denied` | sandbox test **survived** (finding 6); gpu::metal::tests::tally_reads_never_lists_the_caller_as_denied |
| B5 `tally_reads` does not count zero balances as read | gpu_process_listing_under_sandbox_profiles (S) |
| T1 test: S child not under `bash` | gpu_process_listing_under_sandbox_profiles (S line `err=ProcessListDenied`) |
| T2 test: S profile without `same-sandbox` | gpu_process_listing_under_sandbox_profiles (S) |
| T3 test: L profile reduced to `(allow default)` | gpu_process_listing_under_sandbox_profiles (L line `ok denied=0`) |
| T4 test: P profile that fails to parse | gpu_process_listing_under_sandbox_profiles (empty line; stderr `unbound variable`) |
| T5 test: child probe assertion removed, run with `HMN_GPL_CHILD=1` | **passes**, which shows the assertion is load-bearing; unmutated, the same run exits 101 with `HMN_GPL_CHILD set outside a sandbox` |
| T6 test: parent probe assertion removed, run under `sandbox.sh P` | still fails (`sandbox_apply: Operation not permitted`, empty line); unmutated it fails on the parent's message |
| T7 test: the child filter matches nothing | gpu_process_listing_under_sandbox_profiles (empty line) |

B1–B3 show that the Metal arm's wiring is pinned only by the `#[ignore]`d sandbox test, so no CI job sees it. That is the leaf's design (macOS by hand on hardware). It is worth stating in the PR body as a residual, next to the kinfo review's W1–W8.

## Answers to the brief's specific attacks

- **Linux and Windows path.** Yes, `gpu_process_listing_has_no_denied_pids_off_macos` is vacuous on a Linux runner with no GPU (`Err(NoGpuSource)`). Windows is unverified. Nothing else pins `denied_pids` empty: the `listing_without_denials(` grep counts call sites, not the body (W6′ and W7 survive). See finding 1.
- **Sandbox test.** I could not make it pass with a profile that did not apply. A parse failure (T4), a profile weakened to allow-all (T3), the missing `same-sandbox` clause (T2), the missing `bash` sibling (T1), an empty filter (T7), an inherited `HMN_GPL_CHILD` (gate: `inherited exit 101 1`) and a sandboxed parent (gate: `sandboxed exit 101 1`) all fail it. Its gap is the assertions it makes once the profiles do apply (findings 2 and 6).
- **Signature guard.** Yes, a generic parameter (`impl Into<u32>`, `<I: Into<u32>>`) passes it. Literal calls in the integration tests catch those changes. See finding 5.
- **Display test.** No, the list is not complete enough: "outside" and "as root" both pass, and so do others. See finding 3.
- **Doctests.** Yes, the `compile_fail` doctest catches a struct that is not `#[non_exhaustive]` on stable (N1). It also passes for any other compile error (finding 10).

## What I checked and found sound

- **Baseline.** At c456939:
  - lib `gpu:: error::` 77 passed;
  - both `GpuProcessListing` doctests `ok`, on separate `test result:` lines as the gate expects;
  - `macos_sandbox -- --ignored --nocapture` passes, with five `LISTING` lines: S `ok denied=1018 entries=0`, S0/P/L `err=ProcessListDenied denied=1018`, unsandboxed `ok denied=0 entries=24`;
  - the inherited run gives `inherited exit 101`, and the run under P gives `sandboxed exit 101 1`.
- **Red commit 38c151b.** Gives `1 passed; 8 failed` in the lib, and the sandbox test fails on its first profile S (`LISTING ok denied=0`). There are 9 `panicked at` and 0 `error[E`. The signature guard is the one pass.
- **Every added `#[allow]` fires.** The three `#[allow(clippy::expect_used)] // test-only` in `tests/macos_sandbox.rs` are the only added allows. Swapped for `#[expect]`, `cargo clippy --locked --all-targets -- -D warnings` gives rc 0 with 0 unfulfilled under:
  - stable, default features;
  - stable, `--all-features`;
  - stable, `--all-features --target x86_64-unknown-linux-gnu` (the file is cfg'd out there, so the expectation is moot);
  - `+1.88`, default features;
  - `+1.88`, `--all-features`.

  Each of the three fns calls `.expect`. The two vacuous allows on `gpu_processes` were dropped, as amendment A5 asked.
- **House idiom.**
  - No `cfg!`, `panic!`, `unreachable!`, `_ =>` arm or `as` cast in the added test code.
  - The only `.expect(` calls are in the three allowed fns.
  - `#[cfg(not(target_os = "macos"))]` comes before `#[test]`.
  - The ignore reason starts with "requires".
  - The non-macOS statement of `gpu_processes_returns_result_or_no_gpu_source` is unchanged.
  - `Err(other)` in `listing_line` is a binding arm, which is required because `HypomnesisError` is `#[non_exhaustive]` from outside the crate.
  - The decision tests read results with `.ok()` and `matches!`, never with `unwrap`.
  - `cargo fmt --check` exits 0.
- **Feature sets.** `cargo check --no-default-features` and `--no-default-features --features nvml,dxgi,pdh` each give rc 0 with 0 warnings. `cargo test --no-default-features --lib --no-run` gives rc 0, so `sort_by_pid`'s added `test` cfg works. `cargo clippy --lib --no-default-features -- -D warnings` gives rc 0.
- **Not tautological.** The decision tests write literal inputs and outputs (900, 908, `vec![3, 5, 9]`, `denied: 3`), and none restates production constants. The macOS caller test is not vacuous unsandboxed: W3 and W4 fail it.
- **Prototype fixes.** Findings 1–4 were prototyped together (`scratchpad/vgt_proto.diff`). With them, the lib gives 79 passed and the sandbox test passes. Clippy is clean on stable and on the Linux target. They catch W6′, W5, X2 and P4 respectively. The prototype was reverted.
- **Clean-up.** Both worktrees were clean after every mutation, and both are removed.
