# PR C consistency pass (hypomnesis#8, `v0214-part2` @ c026a93)

Read-only review. I made no edits, commits or pushes. I built in a detached worktree and removed it afterwards.
Paths are relative to the checkout `<worktrees>/v0214-part2`.
Line numbers are the HEAD c026a93 line numbers.

## 1. Summary

PR C is close to the maintainer's house style.

- On the mechanical rules it is clean.
  - `cargo fmt --check` and `cargo clippy --all-targets` pass under default features, under `--no-default-features`, and under `--no-default-features --features cli`.
  - `RUSTDOCFLAGS=-D warnings cargo doc` passes.
  - Every new `unsafe` block has a `// SAFETY:`, every `as` has a `// CAST:`, every new `pub` struct and enum is `#[non_exhaustive]`, and the `# Errors` bullets follow the `Returns [..] on/if/when` form.
  - The retry arm carries an `// EXPLICIT:` annotation.
  - The `unsafe` table in CONVENTIONS.md was updated.
  - Test modules carry only `#[allow(clippy::unwrap_used)]`.
  - Pure deciders take closures, as `decide_exists` does.
- The drift risk is in four places.
  - **Remedy join.** Three sites each build "text, then ` — `, then `remedy_text(.., Names)`".
  - **`--pid` rule.** The `--pid` membership test is written twice in `PsFilters`, and the doc says so.
  - **Errno types.** Errno is typed three ways in `metal.rs`: `Option<i32>`, `i32` with 0 meaning none, and `Err(0)`. It is read through three spellings.
  - **Restated rules.** Several rules are restated in 4 to 6 places: the `ProcessListDenied` trigger, the "failed fill is never parsed" rule, "cut at 16 bytes", and "hmn lists itself at 16 KiB".
- Two PR-B-style conventions were not carried over.
  - The family `kern_proc_pid_*` gained a sibling spelled `kinfo_all_*`.
  - NVML named its retry and size caps (`NVML_RETRY_MAX_PROCESSES`), but the PR has a bare `0..4` and a bare `/ 8`.
- One error string departs from the convention's `<noun> <problem> (<context>)` shape.
- None of the proposals needs a behaviour change, except one error-wording row (row 9) that I flag.

## 2. Findings table

Ranked in the order you gave: consolidation and drift surfaces, then conventions, then idiom.
Sizes are approximate net lines.
"Opinion" means I found no maintainer precedent to cite.

| # | file:line | finding | maintainer's precedent | proposed change | kind | size |
|---|---|---|---|---|---|---|
| 1 | `src/bin/hmn/format.rs:204` (`failure_detail`), `src/bin/hmn/ps.rs:599` (`remedy_clause`), `src/bin/hmn/watch.rs:1001` (`denied_pid_notices`) | Three sites each write `format!("{text} — {}", remedy_text(outside_sandbox, RemedyPurpose::Names))`. The separator, the purpose and the order are spelled three times. The `watch.rs:1144` call `remedy_clause(0, denied.len(), ..)` also passes a dummy `0` for the protected count, which reads as a misfit. | PR B's `remedy_text`/`RemedyPurpose` exists so that "the one remedy" is one item. `format.rs:176-193` states the point of the design. `d00d034`, `767ab57` and `32ae825` consolidate in the same way. | Add `pub fn with_remedy(text: &str, outside_sandbox: bool) -> String` in `format.rs` beside `remedy_text`. Call it from the three sites. Optionally give `remedy_clause` a second constructor or an `unreadable`-only entry so `watch` does not pass `0`. | consolidation | +8 / -9 |
| 2 | `src/bin/hmn/ps.rs:268-276` (`judge`) and `ps.rs:292-298` (`relevant_denied`) | The `--pid` rule `pids.is_empty() \|\| pids.contains(&pid)` is written twice. The rustdoc of `relevant_denied` admits it: "this is `judge`'s `--pid` test and nothing else". That is a drift surface by the author's own account. | `c0982d8` "one PsFilters value for --pid/--device/--min". `767ab57` "shared footprint predicate". `footprint_bytes` is shared by `judge`, the comparator and `hmn watch`. | Add `fn pid_selected(&self, pid: u32) -> bool` on `PsFilters`. `judge` and `relevant_denied` both call it. Drop the "nothing else" sentence. | consolidation | +6 / -4 |
| 3 | `src/gpu/metal.rs:506`, `:852`, `:762`, `:871` | Errno is typed three ways in one file. (a) `Option<i32>` in `footprint_from_errno` and `libproc_outcome`, through the new `last_errno()`. (b) `i32` with 0 meaning none in PR B's `PathLookup::Failed { errno }` and `classify_*` (`kinfo.rs`). (c) `Result<String, i32>` with `Err(0)` for "no errno" in `read_proc_pidpath_basename` and `name_after_pidpath`. It is read through three spellings: the inline `std::io::Error::last_os_error().raw_os_error().unwrap_or(0)` at `metal.rs:707` and `:745`, `last_errno()`, and `last_errno().unwrap_or(0)`. The `rc == 0 ? 0 : read errno` block is written twice (`metal.rs:742` and `:998`). | PR B's convention is plain `i32`, 0 when `rc == 0`: `kinfo.rs` `classify_kern_proc_pid(rc, errno: i32, ..)`, `PathLookup::Failed { errno: i32 }`. | Keep one rule: `fn errno_after(rc: i32) -> i32` (0 on success). Use it in `proc_pidpath_lookup`, `kern_proc_pid_raw`, `list_kinfo_all` and `read_graphics_footprint`. Make `footprint_from_errno` and `libproc_outcome` take `i32`. Drop `last_errno() -> Option<i32>`. The tests' `None` cases become `0`. | consolidation | -12 net |
| 4 | `src/gpu/metal.rs:962` (`for _ in 0..4`), `:939-945` (`probed / 8`) | Two unnamed constants: the retry count 4, and the 1/8 slack. The 4 appears again in the rustdoc ("4 attempts in all"). | `nvml.rs:72-78`, `:629-640`: `NVML_ERROR_INSUFFICIENT_SIZE`, `NVML_RETRY_MAX_PROCESSES`, each with a doc comment. The maintainer's review of PR B (`01-reviews_v0.md`) asks for "named consts, derived lengths". | `const KINFO_ALL_ATTEMPTS: usize = 4;` and `const KINFO_ALL_SLACK_DIVISOR: usize = 8;`, each documented. The loop and the rustdoc refer to the const. | consolidation | +8 |
| 5 | `src/gpu/metal.rs` (`classify_kinfo_all`, `KinfoAttempt` in `kinfo.rs:92`, `list_kinfo_all`, `trust_kinfo_listing`, `kinfo_all_buffer_len`) | One family has two prefixes. PR B's `KERN_PROC_PID` functions are `kern_proc_pid_raw`, `kern_proc_pid_lookup`, `kern_proc_pid_comm` and `classify_kern_proc_pid`. The `KERN_PROC_ALL` siblings are spelled `*_kinfo_all`. | PR B naming, accepted. The constants are `KERN_PROC_PID` and `KERN_PROC_ALL`, so the functions should track them. | Rename `classify_kinfo_all` to `classify_kern_proc_all`, `list_kinfo_all` to `list_kern_proc_all`, `kinfo_all_buffer_len` to `kern_proc_all_buffer_len`, and `KinfoAttempt` to a `KernProc*`-style name. `trust_kinfo_listing` can stay. | naming | ~25 sites, rename only |
| 6 | Drift of rustdoc: the `ProcessListDenied` trigger. `src/error.rs:94-102`, `src/gpu/mod.rs:389`, `:409`, `:426`, `:521-525`, Display `error.rs:105` | The rule "at least one process refused and none other than the caller's readable" is written 5 times in prose, plus the Display string. The "caller's own row is not returned on its own; hmn lists itself at 16 KiB" argument is written twice (`mod.rs:415`, `:523`). `snapshot.rs:223` and the `gpu_process_listing` doc in `mod.rs` both say "Report the count … no 'outside' to re-run in". | The convention requires one `# Errors` bullet per path, so two bullets stay. Beyond them the maintainer states a rule once and links, as `GpuProcessEntry` docs do for the Windows semantics ("see [`GpuQuerySource::Pdh`] doc-comment"). | The rule lives on `HypomnesisError::ProcessListDenied`. The table cell, `decide_listing` and `snapshot.rs` say "see [`HypomnesisError::ProcessListDenied`]". The 16 KiB argument lives in `decide_listing` only. "Report the count" stays in `gpu_process_listing` only. | doc | -10 |
| 7 | Drift of rustdoc: "a failed `KERN_PROC_ALL` fill is never parsed". `src/gpu/kinfo.rs:53-58` (`ENOMEM` const doc), `kinfo.rs` `classify_kinfo_all` doc, test comment in `kinfo.rs` ("A short fill copies …"), `metal.rs` `list_kinfo_all` doc, `kern_proc_pid_comm` doc | The same reasoning is written in 5 places. The const doc for `ENOMEM` now explains both `sysctl` variants. | PR B's `ESRCH` doc says what the constant is and which callers answer it (`kinfo.rs:47-50`). The reasoning lives in `classify_kern_proc_pid`'s doc. | Restore the `ENOMEM` doc to "what it is, who answers it". Keep the rule in `classify_kinfo_all` only. `list_kinfo_all` says "judged by `classify_kinfo_all`" and nothing more. | doc | -9 |
| 8 | "cut at 16 bytes": `src/snapshot.rs:174-178`, `src/bin/hmn/main.rs:236-238`, `:404-406`, `src/gpu/metal.rs:815-819` | The "a pattern cannot match past the cut" rule is repeated in the `--filter` help for `ps` and for `watch`, and also in the `GpuProcessEntry::name` rustdoc. | `main.rs` already says `ps --filter` is "the same rule as `hmn watch --filter`". The `--help` text restates, so there is precedent for restating. | Weak: keep the rule in `GpuProcessEntry::name` and in one `--help` line. Let the `watch` help say "as for `ps --filter`". Optional, user-facing text. | doc | -3 |
| 9 | `src/error.rs:103-106` | `ProcessListDenied`'s Display is `process list unreadable: {denied} refused, none other than the caller's could be read`. CONVENTIONS "Error Message Wording" gives validation failures the shape `<noun> <problem> (<context>)`. The other count-bearing variant is `device index {index} out of range (have {count} devices)`. The colon form is reserved for `failed to <verb>: {e}`. | CONVENTIONS.md "Error Message Wording"; `error.rs:66` | `process list unreadable ({denied} refused, none other than the caller's could be read)`. Update `error.rs:154`, `format.rs:739`, `tests/cli_ps.rs:62` (`DENIAL_TEXT`). | idiom | 4 lines | 
| 10 | `src/bin/hmn/format.rs:166-170` (`RemedyPurpose::Names` doc) | The variant doc still says "The names of protected rows, on `hmn ps`'s summary line". `Names` is now also the remedy of the denial line, the `N unreadable` count and the denied-PID notice. The annotation is stale, and the maintainer flags inaccurate annotations. | The maintainer's review of PR B: "accurate annotations". | One sentence: "…and of the denial and unreadable-count lines." | doc | 2 |
| 11 | `src/gpu/metal.rs:939` (`kinfo_all_buffer_len`) | A pure function with arithmetic (slack, round-up, never zero, saturation) has no unit test. Every other new pure decider has one. | `ps_exit_code` and `every_device_failed` have table tests. `decide_exists`, `classify_*` and `trust_kinfo_listing` are tested. | Add one table test: `0 → 648`, `648 → 648+81 → 1296`, `usize::MAX` saturates and does not panic. | idiom | +10 |
| 12 | `tests/cli_ps.rs:121`, `tests/macos_smoke.rs:281`, `:353-354` | The Seatbelt profile P is typed three times across two files. `macos_smoke.rs:353` retypes P inside `s0`, though the same file builds `q` from `p` with `format!` at `:282`. The child/parent sandbox-exec loop is also written twice in `macos_smoke.rs` (`process_exists_under_sandbox_profiles` and `gpu_process_listing_under_sandbox_profiles`, about 25 lines each). | `tests/common/mod.rs` exists for helpers shared by several integration tests; its header lists the files that pull it in. `q = format!("{p}…")` at `macos_smoke.rs:282` is the in-file precedent. | Put `PROFILE_P` and a `run_under_profile(profile, test_name, env_var, wrap_in_bash) -> String` in `tests/common/mod.rs`, with `s0` built from P. Extend the header there. The two harness tests become table loops over it. | consolidation | -25 / +12 |
| 13 | `src/bin/hmn/watch.rs:1226-1241` | Two new stderr strings are built inline in `run_watch`: the `because` suffix and the "they are not followed" line. It is also the only `&& let` chain in the crate. Every other attach notice is a pure function returning `Vec<String>` or `Option<String>`, with a test: `missing_pid_notices`, `unmatchable_notices`, `spilling_at_attach_notice`, `format_followed_set_change`. | The neighbours listed in the previous column, at `watch.rs` ~960-1100. | `fn follow_new_unreadable_notice(device: u32, clause: Option<&str>) -> Option<String>`, with a two-case test. Gate on `selection.follow_new && !watched.is_empty()` in the caller. `denied_pid_notices` already follows this pattern. | idiom | +14 |
| 14 | `src/bin/hmn/watch.rs:1284` | The mid-run `sample failed … ({e})` line prints the raw error. `failure_detail`'s doc promises that "the skip line, the `--device` line and the attach error … cannot word the denial differently". That is true of those three, but a reader sees an exception with no comment. | The review comment style in this file ("The sandbox does not change mid-run", `watch.rs:1139-1141`). | One comment at `:1284`, "raw `{e}`: attach already said the remedy once". | doc | 1 |
| 15 | `src/bin/hmn/ps.rs:594-598` (`remedy_clause` doc), `:618-662` (`format_ps_summary` doc) | The clause's shape (`U unreadable, M protected — <remedy>`) is spelled in `remedy_clause`'s doc and again in four places in `format_ps_summary`'s doc. | `format_ps_summary`'s doc already restates `remedy_text`'s words (`:622-624`, from before the PR), so this extends a pattern. | Optional: `format_ps_summary` says "see [`remedy_clause`]" and drops its second statement of the shape. | doc | -6 |
| 16 | `src/bin/hmn/main.rs:342-344`, `tests/macos_smoke.rs:19-22` | Two doc paragraphs were edited without re-wrapping. A 95-column line sits in the middle of `watch`'s help (`main.rs:344`), and a ragged `//!` paragraph in `macos_smoke.rs`. | Both files wrap at about 78. | Re-wrap. `cargo fmt` does not touch doc comments. | doc | 6 (reflow) |
| 17 | `src/lib.rs:15` | The macOS cell of the "GPU-process listing" row says `proc_listpids` + `ledger` + `proc_pidpath` and omits the `KERN_PROC_ALL` fallback. The "Fallback" row names it only for lookups. | `lib.rs` table rows track backend sources. | Add "(`sysctl KERN_PROC_ALL` where libproc is refused)". | doc | 1 |
| 18 | Opinion (no precedent): `src/gpu/metal.rs:1051` (`kern_proc_pid_comm`) | `classify_kern_proc_pid` parses the record, then `kern_proc_pid_comm` parses the same bytes again, because `PidLookup::Record` drops it. | none | Leave. Mentioned only because it is the one place the parser runs twice on one buffer. | idiom | 0 |
| 19 | Opinion: `src/gpu/metal.rs:692`, `:762` | `proc_pidpath_lookup` (PR B) and `read_proc_pidpath_basename` call the same FFI with the same buffer and SAFETY text. `kern_proc_pid_raw`/`kern_proc_pid_lookup` show the maintainer's raw-plus-classify split, but PR C did not create the duplication. | `kern_proc_pid_raw` + `_lookup` (PR B) | Leave, or share a `proc_pidpath_raw` later. Not worth churn in PR C. | consolidation | opinion |
| 20 | Opinion: `kinfo::EPERM` is used qualified (`metal.rs:507`, `:854`, `:807`) while its siblings (`CTL_KERN`, `KERN_PROC_ALL`) are imported by name at `:36-40` | Import style differs within one file. | none | Import `EPERM` by name, or leave it for the call-site clarity. | naming | 1 |
| 21 | Opinion: the exit-code rule is stated in `main.rs:275-280`, `ps.rs:395-412` (`ps_exit_code`), `ps.rs:429-447` (`run_ps`) and the `tests/cli_ps.rs` docs | Four prose copies, all updated together by this PR. | The base already had three of them (`4495484`, `3e7f5d2`). | No change. Recorded as a known drift surface that the maintainer chose. | doc | 0 |

## 3. Respected

- **FFI return codes and errno.**
  - `ESRCH`, `ENOMEM` and `EPERM` are named consts in `kinfo.rs` with `<errno.h>` docs, like PR B and like `NVML_ERROR_INSUFFICIENT_SIZE`.
  - `EPERM` is gated like its `KERN_PROC_*` siblings.
  - Tests use literals on purpose ("so a wrong `kinfo::EPERM` fails here"), which is the maintainer's pattern for pinning constants.
- **Classify with the call outside.**
  - `classify_kinfo_all` reads `rc` first and never reads `buf` or `len` of a failed call, like `classify_kern_proc_pid`.
  - `trust_kinfo_listing` and `name_after_pidpath` take `impl FnOnce`, like `decide_exists`.
- **Annotations.**
  - Every `unsafe` block has a `// SAFETY:`.
  - Every `as` has a `// CAST: from → to, reason`.
  - `KinfoAttempt::Retry => {}` has `// EXPLICIT:`.
  - `FootprintRead::Gone => {}` has `// EXPLICIT:`.
  - New `.to_owned()` and `.to_vec()` in production code carry `// BORROW:`. Test code carries none, which matches `ps.rs` and `watch.rs` tests (11 and 6 `to_owned()` with no BORROW).
  - No direct slice indexing was added: `.get(..)` is used throughout.
- **Public types.**
  - `GpuProcessListing` is `#[non_exhaustive]`, derives `Debug, Clone` like `GpuProcessEntry`, and has documented fields.
  - The `# Errors` bullets use `Returns [..] on/if/when`.
  - `listing_without_denials` is `const fn`, and the new `#[must_use]` functions carry the attribute.
  - `HypomnesisError::ProcessListDenied` carries structured fields and the enum doc names it.
- **`cfg(any(.., test))` gating** on `decide_listing` and `sort_by_pid` follows `status_tgid` and the `spill.rs` pattern.
- **Test module allowances.** Only `clippy::unwrap_used`, and only where used, per "Test-Module Lint Allowances".
- **Test naming and layout.**
  - Sentence-like test names.
  - Section headings `// --- name ---` and sub-headings `// -- name --` both occur in the base.
  - A full table test for `ps_exit_code`, all 16 cells.
  - Per-purpose fixtures (`unreadable(n)`, `unnamed(n)`) in the style of the existing ones.
- **`CONVENTIONS.md`.** The `unsafe` table row now names `KERN_PROC_ALL` and the parser in `kinfo.rs`. No new rule was needed.
- **Mechanical gates.** `cargo fmt --check`, `cargo clippy --all-targets` (default, `--no-default-features`, `--no-default-features --features cli`) and `cargo doc -D warnings` all pass.
- **Single source of the remedy words.** `remedy_text` in `format.rs` is the only place that spells the remedy, apart from test literals and rustdoc examples.

## 4. Behaviour

- **Rows with no behaviour change:** 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 and 21.
  - Rows 3 and 4 change types or add named consts. The retry count (4) and slack (1/8) keep their values.
  - Row 13 moves two strings into pure functions with identical output.
  - Rows 1 and 2 build the same strings and make the same judgements.
  - Row 11 adds a test and changes no code.
- **Row 9 changes a user-visible string.**
  - `HypomnesisError::ProcessListDenied`'s `Display` changes from `process list unreadable: N refused, …` to `process list unreadable (N refused, …)`.
  - It propagates to the `hmn ps` skip line, the `--device` line, `hmn watch`'s attach error and their tests.
  - Library callers who match on the text will see it; the variant is new in this PR, so nothing released depends on it.
  - If the maintainer wants the colon, drop row 9 and leave the rest.
- Row 5 is a rename. It touches no public name, because `KinfoAttempt` and the functions are `pub(super)` or private.
