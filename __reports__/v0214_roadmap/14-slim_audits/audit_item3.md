# Audit, R01 item 3: `src/gpu/metal.rs` and `src/gpu/kinfo.rs`

Branch `v0214-part2`, `git diff b716087..HEAD`. Read-only analysis; no build or prototype was run (the one attempt to make a scratch worktree hit a classifier error, and the analysis does not need it). Line counts are from the files; the "after" figures are estimates.

"R01" is `docs/roadmap-v0.2.14.md`, section *Design decisions taken before starting*. Its own text says what is agreed: one lookup rule (libproc first, `sysctl kern.proc` only when libproc refuses); the four-outcome read; `p_comm` only on `EPERM`, never `ESRCH`; and "two guards stand against a layout that differs: the whole-records length check and the PID cross-check". It also says the `KERN_PROC_ALL` fallback "gets names in the same pass".

## Summary

The branch delivers every R01 behaviour, and the behaviours themselves are right. What it adds on top is seams, not behaviour. Of the metal.rs growth of +982 lines (code +261, comments +152, tests +549), about half could go without losing any R01 behaviour, and none of the cuts lowers safety.

Four pieces go past R01 and past the house shape:
- `trust_kinfo_listing`: a third layout guard, which came from the A5 coordinator ("new design, not drift"), not from the maintainer or R01.
- `fill_kinfo_all_with`: closure injection to unit-test a four-iteration loop counter.
- `comm_from_lookup`: its own amendment (A6) records "runtime effect nil".
- The `PidpathFailure` enum: its `Gone` and `Other` arms are indistinguishable downstream.

Beyond that, there is dead duplication: `ProcRef` and the `HashMap` of comms give a second route to a string the per-PID `p_comm` lookup must produce anyway for profile D, and PIDs `<= 0` are filtered in three places.

The pure classifiers (`classify_kinfo_all`, `footprint_from_errno`, `libproc_outcome`, `name_after_pidpath`, `tally_reads`, `comm_to_name`) are the house shape (`classify_kern_proc_pid`, `decide_exists`, `filter_process_rows`) and pin real R01 behaviour. They are over-tested (about 3x the assertions they need) but not over-built.

The maintainer's own FFI code (`nvml.rs`, `proc_name.rs`) uses named return-code consts, inline control flow, one inline retry, and pure helpers only where logic is shared (`filter_process_rows`, `untruncate`). It has no closure injection and no unit test of an FFI loop. `nvml.rs` has 6 tests in 992 lines; `metal.rs` now has 38.

## The central question

### The smallest change that delivers what R01 agreed

```text
kinfo.rs   keep EPERM, KERN_PROC_ALL, classify_kinfo_all (pure, house shape of classify_kern_proc_pid).
           KinfoAttempt becomes { Records, Retry, Failed }: nothing downstream reads Refused vs Failed.
           Drop KinfoRead.
metal.rs
  FootprintRead { Bytes, Denied, Gone, Unavailable }     // R01 verbatim; footprint_from_errno kept
  read_graphics_footprint(pid): ONE fn.
        idx = OnceLock...; let Ok(idx) = usize::try_from(idx) else { return Unavailable };
        ledger(...); if rc != 0 { return footprint_from_errno(last_errno()) }
        ...; u64::try_from(entry.lei_balance).map_or(Gone, Bytes)
  list_libproc_pids() -> LibprocPids { Pids, Refused, Failed }   // EPERM -> Refused, as now
  list_kinfo_all() -> Option<Vec<KinfoRecord>>:
        for _ in 0..KINFO_ALL_ATTEMPTS { probe (rc != 0 -> None); fill into vec of
            kinfo_all_buffer_len(probed); match classify_kinfo_all(rc, errno, &buf, len) {
              Records(r) => return Some(r), Retry => {}, Failed => return None } } None
        // an inline retry, like nvml.rs's sized retry; no closure, no fill_kinfo_all_with
  list_pids() -> Option<Vec<i32>>:      // the lookup rule, 6 lines
        match list_libproc_pids() { Pids(p) => Some(p),
            Refused => list_kinfo_all().map(|rs| rs.into_iter().map(|r| r.pid).collect()),
            Failed => None }
  read_proc_pidpath_basename(pid) -> Result<String, i32>         // Err = errno, 0 for an unusable path
  name_after_pidpath(read: Result<String, i32>, comm: impl FnOnce() -> Option<String>)   // decide_exists shape
        Ok(n) => Some(n); Err(errno) if errno == kinfo::EPERM => comm(); Err(_) => None
  kern_proc_pid_comm(pid): kern_proc_pid_raw + `classify_kern_proc_pid(..) == PidLookup::Record`
        + parse_kinfo_records(..).first().comm   // ~6 lines, no comm_from_lookup
  ReadTally/tally_reads as now (pure, R01's others_read rule) but the only place PIDs <= 0 are skipped
  list_processes: list_pids -> tally_reads(read_graphics_footprint) -> name per found row -> MetalProcessList
```

Layout guards: `parse_kinfo_records`' whole-records check, `classify_kern_proc_pid`'s PID cross-check for every per-PID lookup, and the live anchor tests (`kern_proc_pid_record_matches_the_kernel_for_this_process`, `list_kinfo_all_holds_this_process`). A residual sentence replaces `trust_kinfo_listing`.

### How far the branch is from it

Not far in design, about 2x in volume. The listed behaviours all exist. The excess is concentrated in:
- 5 items to delete: `trust_kinfo_listing`, `fill_kinfo_all_with`, `KinfoRead`, `comm_from_lookup`, `PidpathFailure`/`pidpath_failure`.
- 3 to inline: `read_graphics_footprint_at`, `footprint_from_balance`, `enumerate_pids_with`.
- 1 to remove: `ProcRef` and the comms map.
- About 20 tests that pin a seam, or pin one behaviour two or three times.

Estimated result for `metal.rs`: code +261 → about +165, comments +152 → about +95, tests +549 → about +260. Total +982 → about +520. `kinfo.rs` +134 → about +100.

## Item table

Cost is "code/total" lines for the item, counting rustdoc and comments in the total. Tests are in the second table.

| What (file:line) | Asked? | Improves? | Idiom | Verdict | Δ lines |
|---|---|---|---|---|---|
| `FootprintRead`, metal.rs:487 (7/14) | R01 verbatim (four outcomes) | Yes. Removes the silent zero, since `Denied` is what `ProcessListDenied` counts. | Matches `PathLookup`/`PidLookup`. | KEEP | 0 |
| `footprint_from_errno`, :507 (7/12) | R01 + leaf | Yes: only `EPERM` is `Denied`, so a PID is never "protected" for another reason. | Pure classifier in the `classify_kern_proc_pid` shape; the maintainer asked for exactly this kind in PR B (`ENOMEM` arm). | KEEP | 0 |
| `footprint_from_balance`, :518 (3/6) | Leaf | The negative-balance rule pre-exists inline. | A named fn for a one-line `u64::try_from(..).map_or(..)`: ceremony. | SIMPLIFY: inline it | −6 |
| `read_graphics_footprint` + `_at`, :527 / :548 (46/84) | R01 (four outcomes) | The behaviour yes. The split exists only so `_at(-1, pid)` can be tested without the `OnceLock`. | The base had one fn; nothing in `nvml.rs` splits a function to dodge a static. | SIMPLIFY: merge back (`let Ok(idx) = usize::try_from(idx) else { return Unavailable }`) | −8 code, −8 test |
| `read_proc_pidpath_basename` → `Result<String, PidpathFailure>`, :779 (21/40) | R01 (`p_comm` only on `EPERM`) | Yes, needed to know why `proc_pidpath` failed. | Fine, except the error type. | SIMPLIFY: `Err(i32)` errno | −2 |
| `PidpathFailure`, `pidpath_failure`, :815 / :825 (13/18) | Added in A6 | `Gone` and `Other` are handled identically by `name_after_pidpath`, so the `ESRCH` split is unobservable. The only behaviour is "`EPERM` or not". | An enum whose two arms share a body, plus a const fn: ceremony. | DROP: `name_after_pidpath` matches `errno == kinfo::EPERM` | −13 code, −10 test |
| `name_after_pidpath` (+ `comm` closure), :840 (10/17) | R01 (the PID-reuse reset in `watch.rs`) | Yes: guarantees `p_comm` never replaces a path on `ESRCH`, so names do not flip. | Same shape as `decide_exists(path, impl FnOnce)`, which he accepted. The closure is real laziness (avoids a `sysctl` when a path answered). | KEEP, with `Result<String, i32>` | 0 |
| `comm_to_name`, :857 (11/20) | R01 (16-byte `p_comm` returned as is) | The NUL cut and the 16-byte case are R01. The valid-prefix rescue of a mid-character cut goes past `proc_name.rs`, which returns `None` for a cut char via `String::from_utf8(..).ok()`. | Defensible and 4 lines; useful for a Japanese user. | KEEP (marginal; flag the divergence from Linux in the PR text) | 0 |
| `LibprocPids`, :874 (6/10) | R01 (one lookup rule) | Yes. `Refused` vs `Failed` is the rule: sysctl only on `EPERM`, so a transient libproc error never reads as a sandbox. | House shape. | KEEP | 0 |
| `libproc_outcome`, :890 (15/22) | Leaf | Pins the rule above. It is called three times, twice with dummy args (`Vec::new()`, `None`), which is awkward. | OK as a pure classifier. | SIMPLIFY: keep the `EPERM` → `Refused` test, drop the positive-PID filtering (see `tally_reads`) | −3 |
| `list_libproc_pids`, :930 (37/57) | R01 | Yes. Reads `errno` right after the failing call. | Matches the base function. | KEEP | 0 |
| `ProcRef` + the `HashMap<i32, String>` comms in `list_processes` + the `comm` mapping in `enumerate_pids_with`, :908 / :1257 (about 17 code) | R01 ("names in the same pass") | Second route to a string that `kern_proc_pid_comm` must produce anyway for profile D (libproc ok, `proc_pidpath` `EPERM`: no enumeration record exists). Saves a few `sysctl` calls on rows that are already few. | Two sources for one fact; the maintainer's names have one (`read_proc_name`). | DROP: always `kern_proc_pid_comm(pid)` on `EPERM`. Only unmeasured trade: a profile allowing `kern.proc.all` but denying `kern.proc.pid` would lose names (note that `trust_kinfo_listing` also assumes `kern.proc.pid` is allowed). | −17 code, −10 comments |
| `last_errno`, :919 (3/7) | Leaf | Dedupes six call sites; names the "read it first" rule once. | Fine. | KEEP | 0 |
| `kinfo_all_buffer_len`, :987 (7/13) | Leaf (`kern_proc_pid_raw` had a fixed `[u8;648]`; this is a sized heap fill) | The whole-records buffer is required: a non-multiple buffer makes every listing fail. The two unit tests duplicate what `list_kinfo_all_holds_this_process` already proves live. | Pure helper, fine as a helper. | KEEP the fn, DROP both tests | −11 test |
| `fill_kinfo_all_with` + `KINFO_ALL_ATTEMPTS`, :997 (11/14); kinfo.rs `KINFO_ALL_ATTEMPTS` | Added in A6 (FFI reviewer: "retry unbounded or absent survives all tests") | The retry itself is cheap insurance: the kernel probe already includes 5 spare records and the fill adds 1/8, so it fires only if the table grows by 100+ entries between two back-to-back syscalls. Failure is closed (`None` → `NoGpuSource`, not a silent zero). The closure exists to unit-test a loop counter. | `classify_kern_proc_pid` has no loop and no injected closure; `nvml.rs`'s retry is inline and untested. Past the house shape. | SIMPLIFY: inline `for _ in 0..4` in `list_kinfo_all`, no closure; keep the constant | −11 code, −57 test |
| `list_kinfo_all`, :1022 (44/65) | R01 | Yes; fills via the shared parser. The closure-in-`fill_with` makes it nest one level deeper than needed. | Flatten into the loop above. | SIMPLIFY | −6 |
| `trust_kinfo_listing` + its wiring in `list_processes` (12 + 3 code, 11 doc) | A5 coordinator design ("new design, not drift"); not the maintainer, not R01 | Closes a gap that needs a kernel whose `kinfo_proc` is not 648 bytes, a case the maintainer called "unlikely" in PR B. Even then the whole-records check rejects it 80 times in 81 (for 656-byte records); only a total that is a multiple of 648 passes, and then garbage PIDs reach `ledger`. Also adds a coupling: the fallback now fails unless the caller's own `KERN_PROC_PID` also works, so a sandbox allowing `kern.proc.all` but not `.pid` (the reverse of the measured Codex row; unmeasured) loses the fallback. | R01 lists two guards and the maintainer accepted exactly those, plus documented residuals ("native Intel unmeasured"). Another guard function is the abstraction-for-a-hypothetical he did not ask for. | DROP; replace with one residual sentence (below). Optional 3-line alternative if a guard is wanted: require the caller's own PID to be in the listing, which costs no syscall. | −15 code, −11 doc, −54 test |
| `KinfoRead` (kinfo.rs, 8/12) + `KinfoAttempt::Refused` | Leaf + A6 | Nothing consumes `Refused` vs `Failed`: `enumerate_pids_with`, `trust_kinfo_listing` and `list_processes` all end in `None`. The A6 residual list admits it. | Two enums for one `Option`. | DROP `KinfoRead` (return `Option<Vec<KinfoRecord>>`); `KinfoAttempt` = `{ Records, Retry, Failed }` | −12 kinfo.rs, −4 metal.rs, −8 test |
| `classify_kinfo_all`, kinfo.rs (13/22) | A6 | Yes. The one load-bearing fact: a fill that fails `ENOMEM` has copied whole records and returns `len` 0, so its buffer must never be parsed. | Exact copy of `classify_kern_proc_pid`'s shape: `rc` first, `buf` unread on failure, in `kinfo.rs`, with synthetic-buffer tests. | KEEP | 0 |
| `enumerate_pids_with(libproc, kinfo_all)`, :1102 (25/27) | Leaf (R01 lookup rule) | The rule is real; the function's two closures and the `ProcRef` mapping are not. `decide_exists` takes a value plus one lazy closure. | Two closures where one suffices. | SIMPLIFY: 6-line `list_pids` that matches on the libproc value and calls `list_kinfo_all` lazily | −19 code, −35 test |
| `comm_from_lookup`, :1133 (10/15); `kern_proc_pid_comm`, :1148 (4/8) | A6 | `comm_from_lookup`'s own amendment says its guard has no runtime effect (a refused call's zeroed buffer reads as an empty comm → `None`). The cross-check inside is still worth keeping. | A seam for one `!=`. | DROP `comm_from_lookup`, inline its three lines into `kern_proc_pid_comm`, still through `classify_kern_proc_pid` | −8 code, −12 test |
| `ReadTally` + `tally_reads`, :1155 / :1174 (39/54) | Leaf (R01's `others_read`, "counting zero-balance reads as read") | Yes. This is the code that makes `ProcessListDenied` correct: the caller's own 16 KiB read must not count as another process (the silent zero in a new form). | A pure folding fn, like `filter_process_rows`. | KEEP; make it the only place PIDs `<= 0` are skipped (today also in `libproc_outcome` and in `list_processes`'s `.filter`) | −3 |
| `MetalProcessList`, :1211 (5/13) | Leaf (item 4 contract) | Needed by `decide_listing`. | Plain struct. | KEEP | 0 |
| `list_processes`, :1241 (43/65) | R01 | Yes. After the cuts above it loses the `comms` map and the `self_lookup` closure. | Fine. | SIMPLIFY | −15 |
| `process_exists` and PR B items | PR B | Untouched by PR C. | Untouched. | n/a | 0 |

Doc density: the long rustdoc on `list_processes` and the `libsystem_ffi` block matches the maintainer's own (`list_compute_processes` carries about 40 doc lines). I would not push on comments beyond what falls out of the deletions above.

## Tests, grouped by what they pin

New tests: 37 in `metal.rs` (510 lines in test fns, plus helpers `me`, `one_record`, `comm_of`, `record_bytes`) and 4 in `kinfo.rs` (about 55 lines).

| Group | Tests (lines) | Pins | Verdict |
|---|---|---|---|
| **Four-outcome read** | `footprint_from_errno_*` ×4 (27); `footprint_from_balance_*` ×1 (6) | R01: only `EPERM` is `Denied`; negative balance is `Gone`. The `EPERM` test uses `kinfo::EPERM` on both sides, so a wrong constant value passes it (A6 found this and fixed it only for `pidpath_failure`). | R01 behaviour. Collapse 4 → 1 test with literals 1, 3, 22, 12, `None`; −20 |
| `footprint_unavailable_…` | 1 (8) | The `_at(-1)` seam; the real wiring (`unwrap_or(-1)`) is untested. | Seam. Goes with the merge of `_at`; −8 |
| **Tally** | `tally_reads_*` ×6 (88) | R01: gone is neither read nor denied; the caller is never denied and never counted as another; a zero balance counts as read; `Unavailable` makes the list `None`. | R01 behaviour. Collapse 6 → 3; −45 |
| **Names** | `name_after_pidpath_*` ×3 (35); `comm_to_name_*` ×4 (32) | R01: `p_comm` only on `EPERM`; NUL cut; 16-byte as is; the mid-character cut. The `Cell` flags in the first three are redundant: if the closure returns a distinct `Some`, the return value already shows whether it ran. | R01 behaviour. Merge the first 3 → 1 (−23) and the last 4 → 2 (−10) |
| **libproc rule** | `libproc_outcome_*` ×4 (42) | R01: fallback only on `EPERM`; ESRCH/ENOMEM/EINVAL/`None` are `Failed`; an empty list is `Failed`. | R01 behaviour. 4 → 2; −20 |
| **Enumeration order** | `enumerate_pids_with_*` ×3 (53) | R01: libproc first; kinfo only on `Refused`. | R01 behaviour via a seam; 3 → 1 on the 6-line `list_pids`; −35 |
| **`KERN_PROC_ALL` classifier** | `classify_kinfo_all_*` ×4 in kinfo.rs (55) | The partial-`ENOMEM`-fill-never-parsed rule; `rc` read first; `len` clamped. | R01-adjacent safety; same shape as the PR B classifier tests. KEEP |
| **`KERN_PROC_ALL` glue (seams)** | `kinfo_all_buffer_len_*` ×2 (11); `fill_kinfo_all_with_*` ×2 (57); `trust_kinfo_listing_*` ×2 (54) | Arithmetic the live test already proves; a loop counter; an amendment guard. | Seams only. Drop all 6; −122 |
| **`p_comm` lookup** | `comm_from_lookup_*` ×1 (12); `pidpath_failure_*` ×1 (10); `kern_proc_pid_comm_*` ×2 (22) | A seam with no runtime effect; the literal-errno pin of a classifier that goes; the live `p_comm` and dead-PID checks. | Drop the first two (−22). Keep the live ones. |
| **Live anchors** | `kern_proc_pid_record_matches_…` (25, pre-existing); `list_kinfo_all_holds_this_process` (28); `kern_proc_pid_comm_returns_this_process_name` (18) | The layout on the running kernel and the `p_comm` value, natively and under Rosetta. These are the maintainer's way (his PR B anchor). Each re-writes the same exe/`argv[0]` comparison block (about 12 lines). | KEEP all three. Factor the shared comparison into one helper; −25 |

Groups that pin R01 behaviour: four-outcome read, tally, names, libproc rule, enumeration order, `classify_kinfo_all`, the live anchors. Groups that pin only a seam: `footprint_unavailable`, `kinfo_all_buffer_len`, `fill_kinfo_all_with`, `trust_kinfo_listing`, `comm_from_lookup`, `pidpath_failure`, and the `tally_reads_skips_non_positive_pids` test (it pins the redundant triple filter).

Estimated tests after: about 260 lines, about 20 tests, including the 4 kept in `kinfo.rs`.

## The `KERN_PROC_ALL` record-size gap: code or one sentence?

One sentence.
- **What the gap is.** A kernel whose `kinfo_proc` is not 648 bytes never answers `KERN_PROC_ALL` with `ENOMEM` (the probe is sized in its own records), so the whole-records check is the only guard. It rejects such a listing except when its byte total is a multiple of 648: about 1 in 81 for 656-byte records. Then the garbage `pid` fields go to `ledger`.
- **How likely.** No such kernel exists. The layout is frozen userland ABI, measured on arm64 and under Rosetta. The maintainer himself called the larger-record case "unlikely" and accepted "native Intel unmeasured" as a documented residual. The gap needs that event times 1/81.
- **Cost of the code.** `trust_kinfo_listing` costs 15 code lines, 11 doc lines, 54 test lines and a closure in `list_processes`. It adds a new failure coupling (the fallback now also needs the caller's `KERN_PROC_PID`, an unmeasured combination under a hostile sandbox).
- **Precedent.** PR B accepted "pinned by a run elsewhere" and documented residuals.
- **What to write instead.** One sentence in R01 and in the `kinfo.rs` module docs, ahead of the "Not verified: native Intel" bullet:
  > A kernel whose `kinfo_proc` is not 648 bytes does not answer `KERN_PROC_ALL` with `ENOMEM` as it does `KERN_PROC_PID`; the whole-records check rejects its listing except when the total happens to be a multiple of 648 (about 1 in 81), so the guard is probabilistic on a kernel nobody has measured (native Intel).

  This also fixes a stale R01 sentence: "A record larger than 648 bytes does not fit the buffer, so `sysctl` fails with `ENOMEM`" is true for `KERN_PROC_PID` only.
- **Optional zero-syscall guard.** `records.iter().any(|r| r.pid == self_pid)` before trusting the list, a PID cross-check in R01's sense, which stays near-deterministic. I would not add it either: it is one more failure path, and the live test `list_kinfo_all_holds_this_process` already pins that the caller is in the listing.

## `classify_kinfo_all` and `fill_kinfo_all_with` against PR B's house shape

- **`classify_kinfo_all` matches it.** It is pure, takes `(rc, errno, buf, len)`, reads `rc` first and never reads `buf` on failure, sits in `kinfo.rs`, and is tested with synthetic buffers. It is the `classify_kern_proc_pid` pattern applied to the second MIB, and it carries the one fact that matters for memory safety of the parse (the partial `ENOMEM` fill). The only overreach is its four-way result: `Refused` vs `Failed` is read by nothing (the A6 residual list says so). Collapse to `{ Records, Retry, Failed }`.
- **`fill_kinfo_all_with` goes past it.** PR B's seam stops at the classifier; `decide_exists`'s closure is the essential lazy second lookup, not a loop driver. Injecting `attempt: impl FnMut() -> KinfoAttempt` to unit-test `for _ in 0..4` is a test seam for a loop counter. The maintainer's own retry (`list_compute_processes`, one sized second call) is inline, and no unit test drives it. Inline the loop; the live test covers the happy path.
- **`comm_from_lookup` also goes past it, for the same reason.** Its own amendment (A6, Evidence table) records "runtime effect nil".

## Missing or weaker than agreed

- **R01's "same pass" names are delivered, but redundantly.** Profile D forces the per-PID route anyway (see `ProcRef` above), so the same-pass route is a second implementation of one fact. If the maintainer reads R01 literally, keep `ProcRef`; otherwise delete it and say so in the PR text.
- **R01's `KERN_PROC_ALL` ENOMEM sentence needs the correction above** whichever route is taken.
- **PIDs `<= 0` are filtered in three places** (`libproc_outcome`'s `retain`, `list_processes`'s `.filter`, `tally_reads`'s `continue`), and the leaf's "skipped without a read" is satisfied by one.
- **Not in scope but visible in the diff:** the `OnceLock` caches a failed template resolution (`unwrap_or(-1)`) for the life of the process, so one transient failure makes `Unavailable` permanent. This is pre-existing, not new.

## Changes I would make first, with the rough count after

1. **Drop `trust_kinfo_listing`**, its wiring and its 2 tests; write the residual sentence above and fix R01's `ENOMEM` sentence. −15 code, −11 doc, −54 tests.
2. **Drop `fill_kinfo_all_with` and `KinfoRead`.** `list_kinfo_all` returns `Option<Vec<KinfoRecord>>` with the 4-try loop inline; `KinfoAttempt` becomes `{ Records, Retry, Failed }`. −27 code, −65 tests; flatter FFI function.
3. **One name path.** Drop `ProcRef`, the comms map, `PidpathFailure`/`pidpath_failure`, `comm_from_lookup`; `read_proc_pidpath_basename` returns `Result<String, i32>`; `name_after_pidpath` and `kern_proc_pid_comm` as in the sketch. −38 code, −20 comments, −22 tests.
4. **Merge `read_graphics_footprint_at` back, inline `footprint_from_balance` and `enumerate_pids_with`** (a 6-line `list_pids`); one place skips PIDs `<= 0`. −35 code, −45 tests.
5. **Collapse the surviving tests to one per decision:** errno table (4 → 1), tally (6 → 3), names (3 → 1, 4 → 2), libproc (4 → 2), enumeration (3 → 1), a shared `assert_is_my_comm` helper for the three live tests. About −170 test lines.

After: `metal.rs` about +520 (code about +165, comments about +95, tests about +260), `kinfo.rs` about +100; item 3 total about 400 lines lighter. Every R01 behaviour keeps a test.
