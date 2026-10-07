# Audit item 4: the public API (R01 item 4)

## Summary

The public surface is exactly what the maintainer approved and nothing more: `gpu_process_listing`, `GpuProcessListing { entries, denied_pids }`, `HypomnesisError::ProcessListDenied { denied }`, two re-exports, and `gpu_processes` unchanged in signature. The names, the list (not a count), the `Display` with no remedy, and the "weight on the count" wording in the rustdoc all follow his comment. `#[non_exhaustive]`, a `# Errors` section, and a byte-for-byte `Display` test are his own conventions (CONVENTIONS.md; his `no_gpu_source_display_*` test), so they stay. What went beyond him is everything built to pin a mutation or a seam: a second helper (`process_list_denied`) behind `decide_listing`, a constructor helper (`listing_without_denials`) with its own test and a vacuous off-macOS integration test, a fn-pointer signature test, two doctests that test `#[non_exhaustive]`/re-export, a duplicated per-platform table, a macOS smoke test that restates an invariant the unit tests already pin, and a 231-line self-re-executing harness that parses its own text protocol. The harness is worth having (it is the only check of S, L and the Metal arm's wiring; PR B's test covers `process_exists`, `cli_ps.rs` covers P only), but not in this shape or this file. About 660 lines of diff in my area could be about 430, with every behaviour kept. Verified: I merged `process_list_denied` into `decide_listing` in a scratch worktree; clippy is clean (default and `--no-default-features`), the 3 remaining `decide_listing` tests pass, and mod.rs goes from +46 lines of code and tests to 7+/46-.

## Shape of the API vs the maintainer's comment

Matches exactly: names, `entries`/`denied_pids: Vec<u32>`, `ProcessListDenied { denied }`, `denied: u32`, `gpu_processes` kept and made a thin wrapper, Display states the count and has no remedy, the remedy is a CLI constant, no `test-helpers` builder. Public items added: 1 fn, 1 struct, 1 variant, nothing else (`MetalProcessList` is `pub(super)`). Rustdoc: mod.rs follows his long `# Source priority` style and the `# Errors` section clippy requires; "inside an App Sandbox there is no outside" is his point. Two things read as ours rather than his: the same per-platform table appears twice (mod.rs and snapshot.rs; the table cells are long, e.g. the Windows cell explains PDH/NVML internals); and the `GpuProcessListing` struct doc carries two doctests. He writes `#[non_exhaustive]` as a one-line note ("fields may be added in future releases") on every other struct (`SpillReport`, `GpuProcessEntry`, ...) and none has a `compile_fail` doctest.

## Table

| What (file:line region) | Asked? | Improves? | Idiom | Verdict | Δ lines |
|---|---|---|---|---|---|
| `gpu_process_listing` + `gpu_processes` wrapper + docs (mod.rs) | Maintainer (names, wrapper) | Yes: it is the feature | His (long doc, `# Errors`) | KEEP; shorten the macOS/Windows table cells | ~ -4 |
| `GpuProcessListing` struct (snapshot.rs, 7 code lines) | Maintainer | Yes | His (`#[non_exhaustive]`, Debug, Clone) | KEEP | 0 |
| Its two doctests (`compile_fail,E0639`, `use`) | R01/leaf | Pins `#[non_exhaustive]` and the re-export; A7 admits the first passes for any compile error on stable; the second proves only that a name resolves | No precedent in the crate | DROP; the re-export is in the lib.rs diff, `#[non_exhaustive]` is a convention | -12 |
| Per-platform table in snapshot.rs (duplicate of mod.rs) | R01 ("a per-platform doc table") | Duplicate; two tables will drift | Not his | SIMPLIFY: keep one in `gpu_process_listing`, link from the struct | -9 |
| `ProcessListDenied` variant + doc (error.rs) | Maintainer | Yes | His (matches `DeviceIndexOutOfRange`) | KEEP | 0 |
| Enum-level paragraph "Display carries no remedy ..." (error.rs) | Leaf | Restates the variant doc and the changelog | Wordy | SIMPLIFY: delete (keep the `denied` mention in the field list) | -4 |
| `process_list_denied_display_states_the_count_and_no_remedy` (8 lines) | Leaf, tightened in A7 | Pins the user-visible text byte for byte | His own `no_gpu_source_display_*` does the same | KEEP | 0 |
| `process_list_denied` fn + doc (mod.rs, 12 lines) and its 4 tests (~28 lines, incl. a `usize::MAX`/`u64` saturation test) | Leaf | Wraps `others_read == 0 && denied > 0` and a `try_from`. Three of its four tests duplicate `decide_listing`'s cases. Saturation is unreachable (macOS proc limits are thousands) and a truncating `as` would need a `// CAST:` the reviewer would see | Abstraction for testability, one step too many; he splits when the helper has real logic (`classify_kern_proc_pid`), not for one boolean | DROP; inline the condition in `decide_listing` (measured) | -40 |
| `decide_listing` (mod.rs, ~15 lines code+doc) and 3 tests | Leaf | Yes: this is the one place the decision (a sandboxed caller's own row must not read as `Ok`) is testable off hardware; the tests are about behaviour, not seams | Matches `decide_exists` accepted in PR B and `cfg(any(..., test))` in `spill.rs`/`status_tgid` | KEEP (one function) | 0 |
| `sort_by_pid` gains `test` in its cfg (mod.rs) | Consequence of `decide_listing` being testable everywhere | Needed | His pattern | KEEP | 0 |
| `listing_without_denials` const fn (14 lines with its 3-feature cfg) | A7/leaf | Saves a 4-line literal at 3 call sites; the cfg list must be kept in sync with the callers | Not his | SIMPLIFY: build `GpuProcessListing { entries, denied_pids: Vec::new() }` inline | -5 |
| `listing_without_denials_has_no_denied_pids` (lib test, 12 lines incl. cfg) | A7 | Asserts `Vec::new()` is empty; the only mutant it kills is a deliberate one | Seam test | DROP | -12 |
| `gpu_process_listing_has_no_denied_pids_off_macos` (smoke.rs, 14 lines) | Leaf | Passes vacuously on any runner with no GPU (A7's own finding); on a GPU host it checks a field no code writes | Not his | DROP; the table and the changelog state it | -14 |
| `gpu_processes_keeps_its_signature_and_gpu_process_listing_is_additive` (6 lines) | Leaf | `let _ = (old, new)`: coercion check. The literal `gpu_processes(0)` calls in `smoke.rs`/`live_*.rs` already break on a signature change | Pins a seam, not behaviour | DROP | -7 |
| Widening `smoke.rs::gpu_processes_returns_result_or_no_gpu_source` and `macos_smoke.rs::gpu_processes_returns_metal_rows_for_self` to accept `ProcessListDenied` | Leaf | Necessary: both panic inside a sandbox otherwise | Same as his earlier `NoGpuSource` arms | KEEP (trim the comment growth) | -3 |
| `macos_smoke.rs::gpu_process_listing_never_lists_the_caller_as_denied` (33 lines) | Leaf | Caller-not-denied and no-overlap are already pinned by `tally_reads_never_lists_the_caller_as_denied` in metal.rs and by the sandbox test; unsandboxed (CI) `denied_pids` is empty so it is vacuous | Not his | DROP | -33 |
| `tests/macos_sandbox.rs` (231 lines) | Not by name. He asked for hardware validation (Principle 3) and, in PR B (68007ba), that sandbox tests fail rather than skip; R01/leaf authored the file | Not a duplicate: `process_exists_under_sandbox_profiles` covers `process_exists`; `cli_ps.rs` covers P through the CLI. This is the only test of S (partial, `Ok` with denied), L (ledger-only denial, which the changelog claims exits 2), and of the Metal arm's wiring | Self-re-exec is his accepted idiom; the 3-variant `Expect` enum, `Profile` struct, `field()` string parser and `line_matches` are not (PR B asserts exact lines) | SIMPLIFY (below) | -100 to -150 |
| CHANGELOG, item 4 (Added ~11 lines, Changed ~7) | R01 | The Changed entry restates the Added one and the `hmn ps` Added bullet | His entries are long, but not repeated | SIMPLIFY: fold the Changed entry into Added | -6 |
| Roadmap awk gates (count `ProcessListDenied`, count `listing_without_denials(` in mod.rs) | A7 | Not shipped; they guard a helper that goes away and an arm nobody will write | Process ceremony | DROP with the helper | 0 in repo |
| lib.rs re-exports; mod.rs "five dispatchers", Metal row added to `# Source priority` | Maintainer / leaf | Yes (Metal was missing from the list) | His | KEEP | 0 |

## `tests/macos_sandbox.rs`

Keep the test, change its shape and home. It was not requested by name, but the maintainer's whole ask is "you have the hardware", and the three things only it checks are real: profile S (the `Ok`-with-denied path), L, and the Metal arm's wiring in `gpu_process_listing`, which A7 lists as a residual. A second copy of PR B's test it is not: the observable differs. The code is too big for what it checks.
- Have the child print one label from a small classifier (`open` / `partial` / `denied` / `caller_denied` / `{other:?}`) and let the parent `assert_eq!` it against a tuple table `(name, profile, under_bash, label)`. That removes `Expect`, `Profile`, `field`, `line_matches`, and the `entries=`/`overlap=` fields (the overlap invariant is pinned in metal.rs and by the caller check in the classifier).
- Drop profile P (the CLI test covers it; S0 has identical expectations).
- Put it in `macos_smoke.rs` next to `process_exists_under_sandbox_profiles`, reusing `sandbox_can_apply_a_profile`, which is otherwise a third copy of 17 lines. The only reason for a separate target was to keep an ignored count recorded in a roadmap gate, which is process, not code. Expected: ~80 lines added to the existing file, instead of a 231-line new file with its own cfg header.

## Amendment A7 additions, proportionate?

- `listing_without_denials_has_no_denied_pids`: no. It kills a deliberate mutant of a constructor that disappears when inlined.
- awk pins: no, they are roadmap gates on that same helper and on a call-site count.
- Display byte for byte: yes. It is his own idiom (his `NoGpuSource` test is byte for byte), 8 lines, and the text is what users read. Keep.
- Saturation input (`usize::MAX`, `u64::from(u32::MAX) + 1`): no. It guards `unwrap_or(u32::MAX)` against a cast that the conventions already flag; the count cannot reach it. Drop with `process_list_denied`.

## `decide_listing` vs `process_list_denied`

One function would do. `process_list_denied` is a one-line predicate plus a `try_from`; the second layer exists so that a number-in, number-out test can run, and three of those four tests are re-statements of `decide_listing` cases. `decide_listing` is the right size and the right seam (it holds the decision the feature depends on, and the pure inputs are what the unit tests build). I tried the merge: 3 `decide_listing` tests, clippy clean, -39 lines.

An optional further move, outside my area: the decision could live next to `tally_reads` in metal.rs and drop `MetalProcessList.others_read` and the `cfg(any(..., test))` on three functions. I would not do it; the cross-platform test is worth more than the field.

## Missing or weaker than agreed

- The Metal arm's wiring (decision skipped, `others_read` passed as 0, `denied_pids` dropped) is caught only by the ignored sandbox test, which no CI job runs. Already a stated A7 residual; keep the sentence in the PR body. The simplified harness keeps this coverage.
- Nothing in the maintainer's list is missing from the API.

## First changes, in order, and the result

1. Merge `process_list_denied` into `decide_listing`, drop its four tests and the fn-pointer signature test (-40; measured).
2. Reduce `macos_sandbox.rs` to a label-per-profile test and move it into `macos_smoke.rs` (231 -> ~80, -150).
3. Drop `gpu_process_listing_never_lists_the_caller_as_denied`, `gpu_process_listing_has_no_denied_pids_off_macos`, and `listing_without_denials` with its test (inline the literal) (-60).
4. Drop the two `GpuProcessListing` doctests and the duplicate table (-21), plus the enum-level `Display` paragraph (-4).
5. Fold the CHANGELOG "Changed" entry into "Added" (-6).

Area diff ~665 lines -> ~430. No behaviour, public name or documented guarantee changes.
