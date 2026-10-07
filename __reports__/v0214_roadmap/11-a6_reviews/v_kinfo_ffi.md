# Review: `task/kinfo_enumeration` @ 201a1db, the new FFI layer

Scope: `git diff b716087..201a1db -- src/gpu/metal.rs src/gpu/kinfo.rs src/gpu/mod.rs`. Reviewer worktree `scratchpad/v_kinfo_ffi` (detached at 201a1db, removed at the end). Host: M3 Pro, macOS 26.6.2. XNU source fetched fresh with `gh api` from apple-oss-distributions/xnu `main` into `scratchpad/vkf_xnu/` (`kern_sysctl.c`, `kern_newsysctl.c`, `proc_info.c`, `sys_generic.c` (the `ledger` syscall), `osfmk/kern/ledger.c`, `libsyscall/wrappers/libproc/libproc.c`). Probe: `scratchpad/vkf_probe/probe.c` (C, no shared code with `hmn`), run under `sandbox.sh` none, P, S, S0, Q, D, L, C.

## Verdict: PASS WITH NOTES

No memory-safety problem, no UB, no read of uninitialised memory, no unchecked size arithmetic. Every `// SAFETY:` on the new calls holds against the BSD `<sys/sysctl.h>` prototype and XNU. The errno classification matches what XNU returns, as measured under every profile. Counts: 0 blocker, 3 should-fix, 6 notes.

## Findings, most severe first

### 1. should-fix: `list_kinfo_all` has no live test, so a wrong MIB or `namelen` passes every `cargo test`

- **Where:** `src/gpu/metal.rs:1007` (`list_kinfo_all`). Its only coverage is the by-hand harness gate `sandbox.sh S --job`.
- **Defect:** no test calls `list_kinfo_all()`. Profile P cannot tell either: the bridge returns `None` there whether `sysctl` lists the table or not, because every other read is denied.
- **Evidence:**
  - M7a (`let namelen: u32 = 2;`, so the kernel answers `ENOENT`, measured) and M9 (`KERN_PROC_PID` in the MIB, so `EINVAL`) pass `cargo test --locked --all-features -- --include-ignored`: 129 lib tests plus every integration binary, 0 failed.
  - Only `S --job` notices: rc 2, no `gpujob` row.
- **Proposed fix:** add an unsandboxed live test beside `kern_proc_pid_record_matches_the_kernel_for_this_process`. I ran this prototype in my worktree and then reverted it:
  ```rust
  #[test]
  fn list_kinfo_all_lists_this_process_with_its_p_comm() {
      let records = match list_kinfo_all() {
          KinfoRead::Records(records) => records,
          KinfoRead::Refused | KinfoRead::Failed => Vec::new(),
      };
      let mine = records.iter().find(|record| record.pid == me());
      assert!(mine.is_some(), "{} records, none for this process", records.len());
      assert_eq!(mine.map(|record| record.comm.clone()), kern_proc_pid_comm(me()));
      assert!(matches!(
          trust_kinfo_listing(KinfoRead::Records(records), || kern_proc_pid_lookup(me())),
          KinfoRead::Records(_)
      ));
  }
  ```
  It passes unmutated, and fails under both M7a and M9.

### 2. should-fix: `pidpath_failure` has no unit test; swapping `EPERM` and `ESRCH` in it passes every `cargo test`

- **Where:** `src/gpu/metal.rs:819`.
- **Defect:** `footprint_from_errno` and `libproc_outcome` each get one test per errno class. The third classifier, which decides whether a name may come from `p_comm`, gets none: the `name_after_pidpath_*` tests start from a `PidpathFailure` value.
- **Evidence:**
  - M5 (EPERM → `Gone`, ESRCH → `Denied`) passes all 129 lib tests and `--include-ignored`.
  - Harness D: 23 rows, 22 of them `"name":null` (0 at the tip).
  - M6d (`errno` clobbered before `pidpath_failure`) shows the same thing.
- **Proposed fix:** a `pidpath_failure_*` test like `footprint_from_errno_*`:
  - `Some(EPERM)` → `Denied`
  - `Some(ESRCH)` → `Gone`
  - `Some(ENOMEM)`, `Some(22)`, `Some(0)` and `None` → `Other`

  It is `const fn`, so a `const` assertion would also do.

### 3. should-fix: the rustdoc of `list_compute_processes` and the `mod.rs` comment under-state when the bridge is `None`

- **Where:** `src/gpu/metal.rs:1277-1279` and `src/gpu/mod.rs:382-387`.
- **Defect:** both say the bridge is `None` when both enumerations are refused, or when every other read is denied. The code is also `None` in these cases:
  - `proc_listpids` fails with an errno other than `EPERM` (`LibprocPids::Failed`): `sysctl` is never tried;
  - `KERN_PROC_ALL` fails, or is distrusted by `trust_kinfo_listing`, after libproc's refusal;
  - the `graphics_footprint` index does not resolve (`tally_reads` → `None`);
  - `device_index != 0`.

  `list_processes`'s own doc (`:1207-1208`, "neither … enumerated") is closer, but still reads as if `sysctl` were tried after any libproc failure.
- **Proposed fix:** "`None` when no enumeration produced a trusted list (libproc failed; or it refused and `KERN_PROC_ALL` was refused, failed or distrusted), when the ledger index did not resolve, or when the sandbox denied every other process's read."

### 4. note: the doc of `denied_pids` / `ReadTally::denied` / `list_processes` leaves out the non-positive-PID exclusion

- **Where:** `src/gpu/metal.rs:1142-1144`, `:1196-1198`, `:1209`.
- **Defect:** the docs say "excluding the caller and gone PIDs" and "a PID whose read is refused is in `denied_pids`". `tally_reads` also skips PID 0, and `KERN_PROC_ALL` lists PID 0 (`kernel_task`), whose ledger the sandbox refuses (measured: `ledger V2 0: rc=-1 errno=1` under P, S, Q and L).
- **Proposed fix:** the code is right; add "and PID 0" to those three docs.

### 5. note: `KinfoRead::Refused` and `KinfoRead::Failed` are never told apart downstream

- **Where:** `src/gpu/metal.rs:1113` (`KinfoRead::Refused | KinfoRead::Failed => None`), `trust_kinfo_listing:1086-1087`, `list_kinfo_all:1029-1033, 1055`.
- **Defect:** the errno classification in `list_kinfo_all` is correct (EPERM measured under Q and C), but nothing observes it. So mutations M5d (EPERM ⇄ ENOMEM in the fill) and M12 (the probe's errno never read) are undetectable by construction.
- **Proposed fix:** none needed for correctness. Say in `KinfoRead`'s doc that the variant is informational, or merge the two variants. Today the doc implies the distinction carries weight.

### 6. note: the retry, ENOMEM and "never parse a failed fill" logic is verified by reading only

- **Where:** `src/gpu/metal.rs:1012-1064`.
- **Defect:** no seam injects a `sysctl` result, so M1, M1b, M2, M10, M11 and M14 survive every test and the harness. The code is right; XNU and measurement show why:
  - On `ENOMEM`, `sysctl_prochandle` sets `req->oldlen = dp - where` and returns before `req->oldidx += req->oldlen`. `userland_sysctl` then writes back `oldidx` = 0, while the whole records that fit are already copied. Measured with 10- and 10.5-record buffers: `rc=-1 errno=12 len=0`, 10 non-zero records in the buffer.
  - The `rc`-first rule is load-bearing, and not backed up by the size guard. Suppose a fill were refused after a successful probe. The zeroed buffer of `len` = `buf.len()` would parse as PID-0 records, and the self lookup (allowed even under Q, measured) would make the guard trust them. `tally_reads`' `pid <= 0` skip is what would hide that. M3 (parse `buf[..]`, not `..len`) is harmless for the same reason: the slack records are PID 0.
- **Proposed fix:** none required. If wanted, a `fn list_kinfo_all_with(sysctl: impl FnMut(*mut c_void, &mut usize) -> (i32, Option<i32>))` seam would let a unit test pin the bound (4 attempts) and "ENOMEM buffer never parsed".

### 7. note: `kinfo_all_buffer_len` saturates to a non-multiple and then aborts on allocation for an absurd probe

- **Where:** `src/gpu/metal.rs:989-995, 1035`.
- **Defect:** for `probed` near `usize::MAX`, `saturating_mul` yields `usize::MAX`, which is not a record multiple. `vec![0; n]` then panics (capacity overflow) or aborts on OOM instead of returning `Failed`. Only a broken kernel answer can produce such a probe; the measured value is 686 KB.
- **Proposed fix:** none needed. `checked_*` plus `Vec::try_reserve_exact` would turn it into `Failed`.

### 8. note: two FFI docs touched by the diff are slightly off against XNU

- **Where:** `src/gpu/metal.rs:129-130`.
- **Defect:** `proc_listpids` with a null buffer returns `(nprocs + 20) * sizeof(int)` (`proc_info.c:proc_listpids`), not `4 * pid_count`. Measured: 4292 against 4216 filled.
- **Defect:** the `proc_pidpath` doc (`:141-144`) is right for sandbox refusals and exits. But XNU checks `proc_find` before MACF, so a PID that does not exist is `ESRCH` even under a refusing sandbox (measured under P, S, Q and D). And `kernel_task` is `ESRCH` unsandboxed but `EPERM` under P. That case is already documented in `decide_exists`; it is fine, just worth one clause.

### 9. note: libproc can leave a stale `errno` on a 0 return, so "no errno" is not always `None`

- **Where:** `src/gpu/metal.rs:933, 969, 786-787`.
- **Defect:** `libproc.c`'s `proc_pidpath` returns `strlen(buffer)` whenever `__proc_info` succeeds, so an empty path would be 0 with `errno` untouched. `proc_listpids` returns the kernel's byte count, which could in principle be 0 on success. `last_errno()` would then read a stale value, possibly an `EPERM` left by an earlier refused call.
- Neither case happens in practice: the kernel's path on success is non-empty, and `PROC_ALL_PIDS` always lists the caller. Worst case, a spurious `p_comm` name, or a fallback to `sysctl`, never a spurious denial.
- **Proposed fix:** none required; optionally clear `errno` before these two calls.

## The FFI layer against the brief

1. **`KERN_PROC_ALL` call**
   - The declaration `sysctl(name: *mut c_int, namelen: u32, oldp: *mut c_void, oldlenp: *mut usize, newp: *mut c_void, newlen: usize) -> i32` matches the SDK's `int sysctl(int *, u_int, void *__sized_by(*oldlenp), size_t *oldlenp, void *, size_t)` (`<sys/sysctl.h>:796`, BSD).
   - The constants match the SDK headers: `CTL_KERN` 1, `KERN_PROC` 14, `KERN_PROC_ALL` 0, `EPERM` 1, `ESRCH` 3, `ENOMEM` 12, `PROC_ALL_PIDS` 1, `PROC_PIDPATHINFO_MAXSIZE` 4096.
   - The MIB is `[i32; 3]`; `namelen` comes from `mib.len()`; `sys_sysctl` accepts 2..=`CTL_MAXNAME`. The handler sees `namelen` 0 with `cmd == KERN_PROC_ALL`, which its check allows.
   - The probe is safe: `oldp` is null and `probed` is a live `usize` (`fuulong`/`suulong`); the kernel adds `KERN_PROCSLOP`, 5 records (measured 1059 probed, 1054 filled).
   - The fill buffer is a zeroed `Vec<u8>` of `kinfo_all_buffer_len(probed)` bytes, and `len = buf.len()`. On success the kernel writes back `oldidx = dp - where` ≤ `oldlen`, and the code still uses `buf.get(..len)`.
   - On failure, `rc` is read first and the buffer and `len` are never read. On `EPERM`, `len` and the buffer are left untouched (measured under Q and C).
   - The retry is bounded at 1 + `KINFO_ALL_RETRIES` = 4 attempts. `errno` is read before any allocation or `drop` on every path.
   - Every `// SAFETY:` on the probe and the fill is true.
2. **Parser reuse and guard**
   - Only `kinfo::parse_kinfo_records` reads records (no second layout). `kern_proc_pid_comm` goes through `classify_kern_proc_pid` first.
   - `trust_kinfo_listing` rejects every non-648 kernel, because the self lookup sees the same `sizeof_kproc` (one handler, `sysdoproc_callback`):
     - **larger:** the record does not fit the 648-byte buffer, so `ENOMEM` → `Unusable` → `Failed`;
     - **smaller:** `len` ≠ 648, a partial record → `Unusable`;
     - **empty listing:** `Failed`, and the self lookup is not asked;
     - **partial listing:** `parse_kinfo_records` returns `None` → `Failed`.
   - The tests pin these cases (M4b is caught). An unwired guard (M4) is caught only by the static grep gate. M13 (the guard asks PID 1) is caught by nothing, and is equivalent as a size check.
3. **errno, measured with probe.c** (no-error results are written as rc 0)

   | Call | none | P / S / S0 | Q | D | L | C |
   |---|---|---|---|---|---|---|
   | `proc_listpids` (probe and fill) | ok | 0, EPERM | 0, EPERM | ok | ok | ok |
   | `KERN_PROC_ALL` probe | ok | ok | -1, EPERM | ok | ok | -1, EPERM |
   | `KERN_PROC_PID` self / other | ok / ok | ok / ok | ok / -1 EPERM (len 648, buffer untouched) | ok | ok | ok |
   | `proc_pidpath` other / nonexistent / 0 | ok / ESRCH / ESRCH | EPERM / ESRCH / EPERM | as P | as P | as none | as none |
   | `ledger` V2 self / other / nonexistent | ok / ok / ESRCH | ok / EPERM / ESRCH | as P | ok / ok / ESRCH | EPERM / EPERM / ESRCH | as none |

   In XNU, `ledger` returns `ESRCH` from `proc_find` before `mac_proc_check_ledger`, whose error is `EPERM` under Seatbelt. `proc_listpids` returns 0 with errno set when `__proc_info` returns -1.

   `footprint_from_errno`, `libproc_outcome`, `pidpath_failure` and `list_kinfo_all` all map only `EPERM` to denied or refused. `ESRCH`, `ENOMEM`, `EINVAL`, other errnos and "no errno" are never denied. `errno` is read right after every failed call: `last_errno()` is the first expression evaluated in each failure arm.
4. **Casts**
   - The new `namelen` cast and the three casts moved into `list_libproc_pids` carry `// CAST:` and a statement-level `#[allow]`.
   - Swapping each `allow` for `expect` gives clippy `-D warnings` clean on aarch64-apple-darwin, x86_64-apple-darwin and 1.88: every lint named fires.
   - `try_from` could replace the three moved casts (`size_bytes`, `cap_bytes`, `written_bytes`) without an allow. They are moved PR-B-era code, so this is a style point, not a finding.
5. **Rustdoc:** accurate apart from findings 3, 4, 5 and 8. In particular, the `ENOMEM`/`EPERM`/`ESRCH` docs in `kinfo.rs`, `trust_kinfo_listing`, `list_kinfo_all`, `tally_reads` (caller excluded, gone excluded, zero balances counted) and `legacy_entries` match XNU and the code.
6. **Gates I ran at the tip:**
   - `cargo test --locked --all-features --lib`: 129 passed, natively, under `--target x86_64-apple-darwin` (Rosetta) and on 1.88.
   - `cargo test --all-features -- --include-ignored`: all pass.
   - clippy `-D warnings`: clean for macOS and for `x86_64-unknown-linux-gnu`.
   - Harness: S `--job` rc 0 with the `gpujob` row; `ps --device 0` rc 0 under none and rc 2 under P, S0, Q and L; D 22 rows with 0 null names.
   - `hmn ps` takes 0.12–0.29 s under every profile, so ~1050 denied ledger reads cost nothing visible.

## Mutations (each applied alone in my worktree, then reverted with `git checkout`)

`lib` means `cargo test --locked --all-features --lib`. The harness is `scratchpad/vkf_hgate.sh`: release build, then S `--job`; `ps --device 0` under none, P, S0, Q and L; D null names.

| Wrong implementation | Caught by |
|---|---|
| retry unbounded (`for _ in 0_usize..`) | none (no `ENOMEM` on this host; reading only) |
| no retry: `ENOMEM` → `Failed` at once | none |
| parse the partial `ENOMEM` buffer | none (no `ENOMEM` on this host) |
| `len` not clamped: parse `buf[..]` | none, and harmless: the slack records are PID 0, which `tally_reads` skips |
| record-size guard unwired at the call site | static gate `grep -c 'trust_kinfo_listing(list_kinfo_all()'` only |
| guard trusts every self lookup | `trust_kinfo_listing_keeps_records_only_when_self_is_one_648_byte_record` |
| guard asks PID 1, not the caller | none (equivalent as a size check) |
| EPERM ⇄ ESRCH in `pidpath_failure` | **no `cargo test`**; harness D (22 of 23 rows nameless) — finding 2 |
| ESRCH for EPERM in `footprint_from_errno` | `footprint_from_errno_eperm_is_denied`, `footprint_from_errno_esrch_is_gone` |
| ESRCH for EPERM in `libproc_outcome` | `libproc_outcome_eperm_is_refused`, `libproc_outcome_other_errnos_are_failed_never_refused` |
| EPERM ⇄ ENOMEM in `list_kinfo_all`'s fill | none (finding 5) |
| probe's errno never read | none (finding 5) |
| fill's `rc` ignored | none (the probe's refusal comes first under Q and C) |
| failed fill (other errno) parses `buf[..len]` | none |
| libproc phase-2 errno never read | none (phase 1 is the one refused) |
| `ledger` errno read after a 1 MiB allocation | none, and harmless: allocation does not touch `errno` here |
| `ledger` errno read after a failing syscall (`stat`) | `cli_ps::ps_exits_2_with_the_skip_line_when_process_info_is_denied` (`#[ignore]`d); harness P, S0, L rc 0: the silent zero returns |
| `proc_listpids` probe errno read after a failing syscall | harness S `--job` only (rc 2) |
| `proc_pidpath` errno read after a failing syscall | harness D only (22 nameless) |
| `namelen` literal 2 | **no `cargo test`**, ignored ones included; harness S `--job` (rc 2) — finding 1 |
| `namelen` literal 4 (the kernel reads one int past the MIB) | none: the call still succeeds (measured) |
| MIB `KERN_PROC_PID` in `list_kinfo_all` | **no `cargo test`**; harness S `--job` — finding 1 |
| fill buffer not a record multiple | `kinfo_all_buffer_len_is_a_record_multiple_with_slack`, `kinfo_all_buffer_len_of_zero_is_one_record_slack` |
| fill buffer = probed, no slack | `kinfo_all_buffer_len_is_a_record_multiple_with_slack` |
| `kern_proc_pid_comm` skips `classify_kern_proc_pid` | none (a refused call's zeroed buffer has an empty comm → `None`) |
| fallback taken on any libproc failure | `enumerate_pids_with_falls_back_only_when_libproc_says_eperm` |

With finding 1's prototype test added, `namelen` 2 and the `KERN_PROC_PID` MIB both fail `list_kinfo_all_lists_this_process_with_its_p_comm`. The remaining "none" rows are either harmless or need an injection seam (finding 6).
