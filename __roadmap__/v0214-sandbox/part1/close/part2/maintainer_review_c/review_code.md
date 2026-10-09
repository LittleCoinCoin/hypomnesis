# A10 code: Codex test skip and the code nits, folded per owning commit

**Goal**: Do the code half of the maintainer's review of PR C:
- the `KERN_PROC_ALL` test skips, with a printed reason, under a policy that refuses `kern.proc.all`;
- PID 0 is not read;
- a listing where nothing was read, nothing was denied and something failed is `None`, not `Ok(empty)`;
- `denied_pids` is sorted and deduplicated;
- `NoGpuSource` gets the fail-closed sentence;
- `comm_to_name` keeps a valid prefix only at 16 bytes;
- the `// EXPLICIT:` and `// BORROW:` annotations.

Each fix is a `git commit --fixup=<owning sha>` on top of 1c97d88, so autosquash folds it into the commit the maintainer reviewed.

**Pre-conditions**:
- [ ] Amendment A10 approved by the user (2026-10-09, plan approval), source `__reports__/v0214_roadmap/17-amendment_a10_pr_c_review_v0.md`. Decisions: `Failed` also covers the three `rc == 0` anomalies; `// BORROW:` goes on all 14 PR-added sites.
- [ ] Worktree `task/a10_code` at 1c97d88, PR C's reviewed head; `rustup check` reports stable up to date.

**Success Gates**:
- ⬜ [run] **Codex profile.**
  - Setup: `git show 6e7cf099e8:__reports__/v0214_part1/harness/codex.sb > "$SCRATCH/codex.sb"`; `B` is the lib test binary from `cargo test --locked --all-features --lib --no-run`.
  - Command: `bash -c 'sandbox-exec -f "$SCRATCH/codex.sb" "$PWD/$B"; echo $?'`.
  - Before (measured 2026-10-09 at 1c97d88): `101`, `117 passed; 1 failed` (`list_kern_proc_all_holds_this_process`).
  - After: `0`, `0 failed`. With `--exact gpu::metal::tests::list_kern_proc_all_holds_this_process`, stderr holds the skip line naming `kern.proc.all` and `EPERM`.
  - Catches: `cargo test` failing inside Codex's sandbox.
  - Nested `sandbox-exec` returns rc 71 while the desktop "Local sandbox" toggle is on. Stop and report it; do not work around it.
- ⬜ [run] (guard) **Unsandboxed.** The same test passes and prints no skip line, before and after. Catches: a skip that fires outside a sandbox.
- ⬜ [run] **Failed reads.**
  - `footprint_from_errno_only_eperm_is_denied` asserts, with literals so a wrong `kinfo::ESRCH` fails: `Some(1)` → `Denied`; `Some(3)` → `Gone`; `Some(22)`, `Some(12)` and `None` → `Failed`.
  - A new `tally_reads` test: reads that are only `Failed`, the caller's own included, give `None`.
  - (guard) One `Denied` among `Failed` gives `Some`, and so does one other `Bytes(0)` among `Failed`.
  - Mutation: in a scratch copy with the `failed > 0` return removed, at least one test fails (`grep -c 'panicked at'` ≥ 1, `grep -c 'error\[E'` = 0).
  - Catches: an errno other than `EPERM`/`ESRCH` quietly giving `Ok(empty)`.
- ⬜ [static] **PID 0.** `grep -c 'pid <= 0' src/gpu/metal.rs` — before: 1 → after: 0. `grep -c 'filter(|&pid| pid > 0)' src/gpu/metal.rs` — before: 0 → after: 1. Catches: PID 0 read before it is skipped.
- ⬜ [run] **`comm_to_name`.** `comm_to_name_keeps_the_valid_prefix_of_a_cut_char` asserts `comm_to_name(b"ab\xe3") == None`, and the 16-byte Japanese cut still gives `Some("日本語プロ")`. Mutation: with the length condition removed, the test fails. Catches: a short `p_comm` read as a cut.
- ⬜ [run] **Sort and dedup.**
  - The renamed `decide_listing` test passes `denied_pids` `[20, 7, 20]` and expects `[7, 20]`.
  - A row with `[5, 5]` and `others_read` 0 gives `ProcessListDenied { denied: 1 }`.
  - Before: the test pins `vec![20, 7]`, unsorted (mod.rs:864).
  - Catches: a silent `binary_search` miss, and a count that disagrees with the list.
- ⬜ [static] **Annotations.**
  - Count the PR-added `.to_owned()`/`.as_bytes()` lines that have no `// BORROW:` on the same line or in the two lines above. Scope: the files of `git diff --name-only b716087..HEAD -- src tests`, attributed by `git blame` to the range `b716087..HEAD`. Before: 14 (`metal.rs` 8, `ps.rs` 2, `tests/macos_smoke.rs` 4) → after: 0.
  - `tally_reads` carries a `// EXPLICIT:` above its `for`: `EXPLICIT:` between `fn tally_reads` and the next `^}` counts ≥ 2 with the existing `Gone` arm.
- ⬜ [static] **`NoGpuSource` doc.** `grep -c 'KERN_PROC_ALL' src/error.rs` — before: 0 → after: ≥ 1, in the `NoGpuSource` variant doc. (guard) `git diff 1c97d88 -- src/error.rs` changes only `///` lines, so the `#[error(` text and `no_gpu_source_display_names_the_backends_of_the_platform` stay unchanged.
- ⬜ [static] (guard) **Windows and Linux untouched.** `git diff 1c97d88 -- src/gpu/pdh.rs src/gpu/dxgi.rs src/gpu/nvml.rs src/gpu/nvidia_smi.rs src/gpu/proc_name.rs` is empty.
- ⬜ [run] **Trial autosquash.**
  - On a scratch branch from the task branch: `GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash --exec "$GATES" b716087`, where `$GATES` is `cargo fmt --check && cargo clippy --locked --all-targets --all-features -- -D warnings && cargo clippy --locked --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings && cargo test --locked --all-features`. It exits 0.
  - `git rev-list --count b716087..HEAD` = 18, and `git log --format=%s b716087..HEAD | grep -c '^fixup!'` = 0.
  - Report every conflict and its resolution.
  - Catches: a fixup that leaves its owning commit red, or lands in the wrong commit.

**References**: [PR C review](https://github.com/mi-for-the-rust-of-us/hypomnesis/pull/8#pullrequestreview-5453310396), "Requested changes" 2 and "Nits"; [CONVENTIONS.md](../../../../../../CONVENTIONS.md), `// EXPLICIT:` and `// BORROW:`; precedent: `LibprocPids::Failed` and `libproc_outcome` (metal.rs ~832-864), `tests/cli_ps.rs` `accept` (`writeln!` to stderr, since libtest captures `eprintln!`), `pdh.rs:1168` (`// EXPLICIT:` above a stateful loop), `sort_by_pid` (mod.rs ~659), and the base's `list_compute_processes` (`git show b716087:src/gpu/metal.rs`), which skipped `pid <= 0` before the read

**Rules for every step** (the brief repeats them):
- **Commit field.** Each step's **Commit** names the subject of its owning commit, which the step's `git commit --fixup=<sha>` targets. A step whose deliverables span several owners lists the other owners in its Implementation Logic, one `fixup!` each.
- **Owning commit.** Each fix is a `git commit --fixup=<sha>` of the commit that introduced the code or text it fixes (`git blame 1c97d88`). Every squashed commit's docs must be true for its own code. Where a later commit rewrote the same lines (10c6e57, 7e9e4aa, 4f7a0dc), resolve the trial-autosquash conflict to the final text and report it.
- **Spec defects.** Report a spec defect rather than implement it faithfully.
- **Scope.** Add no function, type, guard or test this leaf does not name.
- **Lints.** No `panic!`: `clippy::panic` is denied.
- **Rustdoc.** State the final state, with no "now" and no "no longer".

## Step 1: The KERN_PROC_ALL test skips where kern.proc.all is refused
**Goal**: Under a policy that refuses `kern.proc.all` (Codex), the test prints why it skips and passes; everywhere else it asserts as before.
**Implementation Logic**:
1. In `list_kern_proc_all_holds_this_process`, call `let listing = list_kern_proc_all();`.
2. The next statement: `if listing.is_none() && last_errno() == Some(kinfo::EPERM) { let _ = writeln!(std::io::stderr().lock(), "<skip line>"); return; }`. The line names what was refused (`kern.proc.all`, `EPERM`), like `cli_ps`'s branch lines, with no remedy and no "now".
3. Then `let records = listing.unwrap_or_default();`; the rest stays unchanged.
4. Add `use std::io::Write as _;` to the test module.

`last_errno` is exact on the probe-failure path, which is Codex's case.
**Deliverables**: `src/gpu/metal.rs` (`tests::list_kern_proc_all_holds_this_process`, the test module's `use`)
**Consistency Checks**: `bash -c 'cargo test --locked --all-features --lib -- gpu::metal::tests::list_kern_proc_all_holds_this_process 2>&1 | grep -q "1 passed"'` (expected: PASS)
**Commit**: `feat(metal): enumerate with sysctl when libproc refuses and name denied PIDs from p_comm`

## Step 2: PID 0 is skipped before its read
**Goal**: Read no PID that is not positive, as the base did.
**Implementation Logic**:
1. `list_processes`: `pids.into_iter().filter(|&pid| pid > 0).map(|pid| (pid, read_graphics_footprint(pid)))`.
2. Remove from `tally_reads`:
   - its `if pid <= 0 { continue; }`;
   - the doc sentence on skipping;
   - the `ReadTally.denied` clause "and not a PID that is not positive".
3. Remove the rows `(0, …)` and `(-1, …)` and their comment from `tally_reads_counts_every_read_pid_and_lists_a_row_for_a_non_zero_balance`.
4. The `list_processes` doc says a PID that is not positive "is skipped before its read". Those doc lines belong to 7e9e4aa, so they take a `fixup!` of 7e9e4aa; at each commit, edit the doc where it lives.
5. Leave the `u32::try_from(pid)` conversions as they are.
**Deliverables**: `src/gpu/metal.rs` (`list_processes`, `tally_reads`, `ReadTally`, one test)
**Consistency Checks**: `bash -c 'test "$(grep -c "pid <= 0" src/gpu/metal.rs)" = 0 && cargo test --locked --all-features --lib -- tally_reads'` (expected: PASS)
**Commit**: `feat(metal): enumerate with sysctl when libproc refuses and name denied PIDs from p_comm`

## Step 3: A read that failed is Failed, and only failures give None
**Goal**: Close the silent empty list. `Gone` is `ESRCH` only, and a listing where nothing was read, nothing was denied and something failed falls through to `NoGpuSource`.
**Implementation Logic**:
1. Add `FootprintRead::Failed`, documented like `LibprocPids::Failed`: the read failed, and the cause is neither a refusal nor the process exiting. That covers any other `errno` or none, an empty entry array, an index past it, and a negative balance.
2. `FootprintRead::Gone` means `ESRCH` only.
3. `footprint_from_errno` becomes a `match`: `Some(kinfo::EPERM) => Denied`, `Some(kinfo::ESRCH) => Gone`, `_ => Failed`.
4. `read_graphics_footprint`'s three `rc == 0` anomaly returns give `Failed`.
5. `process_gpu_info`'s arm gains `| FootprintRead::Failed`.
6. `tally_reads` keeps a local `let mut failed = 0_usize;` (no `ReadTally` field), and the caller's own read counts in it. After the loop: `if tally.others_read == 0 && tally.denied.is_empty() && failed > 0 { return None; }`.
7. Docs:
   - the enum, `footprint_from_errno`, `read_graphics_footprint` and `tally_reads`;
   - `list_processes`' "`None` when:" list gains "no read worked, none was refused and at least one failed";
   - the dispatcher comment at `src/gpu/mod.rs` ~434 names that case.

   The `mod.rs` comment and the `list_processes` doc lines 7e9e4aa owns take a `fixup!` of 7e9e4aa.
8. Tests:
   - `footprint_from_errno_only_eperm_is_denied` as the gate says;
   - one new `tally_reads` test for the `None` case, with the two `Some` guard rows.
**Deliverables**: `src/gpu/metal.rs` (`FootprintRead`, `footprint_from_errno`, `read_graphics_footprint`, `process_gpu_info`, `tally_reads`, the `list_processes` doc, tests); `src/gpu/mod.rs` (dispatcher comment)
**Consistency Checks**: `bash -c 'cargo test --locked --all-features --lib -- footprint_from_errno tally_reads && cargo clippy --locked --all-targets --all-features -- -D warnings'` (expected: PASS)
**Commit**: `feat(metal): enumerate with sysctl when libproc refuses and name denied PIDs from p_comm`

## Step 4: comm_to_name keeps a valid prefix only at 16 bytes
**Goal**: Only a 16-byte `p_comm` can have been cut by the kernel.
**Implementation Logic**:
1. The cut arm becomes `Err(e) if e.error_len().is_none() && bytes.len() == kinfo::P_COMM_SIZE - 1`, so a shorter incomplete sequence falls to `Err(_) => return None`.
2. The doc and the in-body comment say that only a name of exactly 16 bytes can have been cut, and a shorter one ending in an incomplete character is not a name.
3. The test: `b"ab\xe3"` gives `None`, with the comment reworded; the 16-byte Japanese case is kept.
**Deliverables**: `src/gpu/metal.rs` (`comm_to_name`, `tests::comm_to_name_keeps_the_valid_prefix_of_a_cut_char`)
**Consistency Checks**: `bash -c 'cargo test --locked --all-features --lib -- comm_to_name'` (expected: PASS)
**Commit**: `feat(metal): enumerate with sysctl when libproc refuses and name denied PIDs from p_comm`

## Step 5: The CONVENTIONS annotations
**Goal**: `// EXPLICIT:` on `tally_reads`' loop, and `// BORROW:` on every `.to_owned()`/`.as_bytes()` the PR added.
**Implementation Logic**:
1. One `// EXPLICIT:` between `let mut tally` and the `for`, with its reason:
   - the loop is a stateful fold;
   - the caller's read counts differently from the others;
   - `Unavailable`, and the failed count, abandon the tally.

   No `fold` rewrite.
2. `// BORROW:`, mirroring the wording at metal.rs ~1209 ("`as_bytes` views … in place") and ~826, at:
   - `metal.rs` tests ~1330, 1333-1334, 1342, 1346, 1357, 1360 and 1364 (6103aac);
   - `src/bin/hmn/ps.rs` ~1055 and 1057 (c07282f);
   - `tests/macos_smoke.rs` `listing_label` (7e9e4aa): one comment above the `match`, covering its four labels.
3. Annotate any `.to_owned()` that Steps 1-4 add.
4. One `fixup!` per owning commit: 6103aac (`metal.rs`), c07282f (`ps.rs`), 7e9e4aa (`macos_smoke.rs`).
**Deliverables**: `src/gpu/metal.rs`, `src/bin/hmn/ps.rs`, `tests/macos_smoke.rs` (comments only)
**Consistency Checks**: `bash -c 'cargo fmt --check && cargo clippy --locked --all-targets --all-features -- -D warnings'` (expected: PASS)
**Commit**: `feat(metal): enumerate with sysctl when libproc refuses and name denied PIDs from p_comm`

## Step 6: denied_pids sorted and deduplicated, and the fail-closed sentence
**Goal**: `denied_pids` is sorted like `entries`, and `NoGpuSource`'s doc names the sandbox cases it covers.
**Implementation Logic**:
1. `decide_listing`: `denied_pids.sort_unstable(); denied_pids.dedup();` before `denied` is computed. Use `sort_unstable`: pedantic's `stable_sort_primitive` fires on a stable sort of `u32`, and `sort_by_pid` takes structs.
2. `decide_listing`'s doc: "the rows and the denied PIDs, each sorted by `pid`".
3. The `GpuProcessListing.denied_pids` doc (`src/snapshot.rs` ~232): "Sorted by `pid` ascending, without duplicates; always empty on Linux and Windows."
4. Rename `decide_listing_sorts_by_pid_and_keeps_denied_pids` to match. Its input `[20, 7, 20]` expects `[7, 20]`; add the `[5, 5]` → `denied: 1` row.
5. `ReadTally` and `MetalProcessList` keep "in enumeration order".
6. `src/error.rs` `NoGpuSource`: one sentence after its backends sentence, in the voice of the existing docs. It says that on macOS, `gpu_processes` and `gpu_process_listing` also return the variant when:
   - the caller's sandbox refuses `proc_listpids`, and no `sysctl` listing can stand in;
   - that is, `KERN_PROC_ALL` is refused, or is allowed while `KERN_PROC_PID`, which vouches for its record size, is refused.

   It ends by saying that the sandbox is then the cause, though the message names the backends. `Display` is untouched.
7. Items 1-5 take a `fixup!` of 7e9e4aa; item 6 takes a `fixup!` of d774660 (`docs(gpu): document gpu_process_listing, its denied PIDs and ProcessListDenied`).
**Deliverables**: `src/gpu/mod.rs` (`decide_listing`, its doc and test); `src/snapshot.rs` (field doc); `src/error.rs` (the `NoGpuSource` doc)
**Consistency Checks**: `bash -c 'cargo test --locked --all-features --lib -- decide_listing no_gpu_source && test "$(grep -c KERN_PROC_ALL src/error.rs)" -ge 1'` (expected: PASS)
**Commit**: `feat(gpu): add gpu_process_listing and ProcessListDenied`
