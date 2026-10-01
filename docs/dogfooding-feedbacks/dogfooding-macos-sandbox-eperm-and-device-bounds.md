# Dogfooding report (from the issue #3 field check on Apple Silicon): v0.2.13 passes all eight checks — but macOS's `ledger` `EPERM` comes from the sandbox, not from process ownership, and a sandboxed `hmn ps` reports `0 GPU processes found` with exit 0

**Date:** 2026-10-01
**Reporter:** field check for issue #3, Apple M3 Pro (arm64), macOS 26.6.2 (25G83), unprivileged uid 501, `hmn 0.2.13` built from `cf5ada0` with `cargo build --release`
**Severity:** validation of `process_exists` and the v0.2.13 `hmn ps`/`hmn watch` output on macOS + request for sandbox-aware reporting, doc, error-path and JSON/text-cell corrections
**Affected area:** `metal::list_compute_processes` on `EPERM`; the macOS cross-user doc claim (`hmn --help` Limitations, `metal.rs` and `gpu/mod.rs` docs); `gpu::bounds_check`; `gpu::process_exists` at PID 0; the SPILL/PAGED cells and the `spilled` field where spill does not exist
**Status:** Proposed — v0.2.14 candidates

---

## TL;DR

All eight checks from issue #3 pass as worded. `cargo test --all-features` exits 0: 338 passed, 0
failed, and both `process_exists` unit tests `ok`. `hmn watch` warns about a dead PID and only that
PID. The spill summary says `not measurable`, the JSON says `"measurable":false`, `--top 5` lines
up, and `ps --filter`, `--exit-status`, `--device 1` and `--help` behave as specified.

The finding is the premise of check 3. The crate documents that macOS **cross-user** PIDs are
skipped because `ledger` returns `EPERM`. Measured, **ownership does not matter; the sandbox
does:**

- Unsandboxed, `ledger` returned rc 0 for 249 of 250 other-user PIDs, root included. The one
  failure was `ESRCH`.
- Under a sandbox that denies `process-info*` on other processes, a **same-user** Safari gets
  `EPERM`.
- In that sandbox, `hmn ps` prints `0 GPU processes found.` and exits 0. `hmn watch` exits 2,
  blaming a missing GPU backend.

The `None` ("can't tell") branch of `process_exists`, which check 3 was meant to exercise, was
reached by the probe but never by `hmn`.

Smaller defects:

- `--device 1` prints the generic `NoGpuSource` text, because `bounds_check` has no Metal branch.
- `hmn watch 0` claims kernel_task does not exist.
- On UMA, SPILL reads `?` and the JSON says `spilled:false`. Both describe something that does not
  apply as if it were unknown or false.

## What worked — eight checks, no surprises in the output

These parts behave exactly as issue #3 describes, so they are the ones not to break.

```
hmn watch: device 0 [Apple M3 Pro], interval 1.0s, watching 2 PID(s)
hmn watch: pid=999999 names no running process; its rows will read 0 MiB
```

That is the whole stderr of `hmn watch 999999 2561 --interval 1s --duration 2s`, where 2561 is
Safari. There is no line for 2561. A dead PID inside the valid range (99998) warns the same way.

```
hmn watch: spill not measurable on this platform; per-PID VRAM below
hmn watch: per-PID  PID   NAME    BASELINE COMMIT  PEAK COMMIT  BASELINE SHARED  PEAK SHARED  PAGED
                    2561  Safari  2 MiB            2 MiB        0 MiB            0 MiB        ?
```

`no spill observed` appears 0 times across every captured run. Parsed with `json`, the `--json`
summary has `"measurable":false` and `"spilling_at_attach":null`, and the samples carry
`"spilling":null,"paged":null`.

```
TIME      PID      NAME                         COMMITTED  ΔCOMMIT    SHARED     ΔSHARED    SPILL
+0.0s     393      WindowServer                 264 MiB    +0 B       0 MiB      +0 B       ?
+0.0s     396      loginwindow                  129 MiB    +0 B       0 MiB      +0 B       ?
+0.0s     2728     com.apple.WebKit.WebContent  56 MiB     +0 B       0 MiB      +0 B       ?
```

`--top 5` has 0 misaligned cells, measured by display width, not by eye. The header is 97 columns
but 99 bytes because of the two `Δ`, so the padding counts `char`s, as it should. Explicit PIDs
widen NAME to fit a 44-char `com.apple.appkit.xpc.openAndSavePanelService`, and the columns stay
aligned.

```
hmn: 1 GPU process found matching filter="sAfArI" (2 MiB committed total).     # exit 0
hmn: 0 GPU processes found matching filter="ZzNoSuchApp".                      # exit 1
```

`hmn --help` puts `Commands:` at character offset 1440 and `Limitations (per-platform):` at 3513.

## What is wrong — `EPERM` is a sandbox verdict, not an ownership one

Issue #3 says that WindowServer's `ledger` read gets `EPERM`, `so this is exactly the case where
process_exists is asked`. The help text says the same (`src/bin/hmn/main.rs:169-171`):

```
- macOS: cross-user PIDs are silently skipped — the per-PID `ledger` syscall returns `EPERM` for
  processes owned by another user. To list every PID on the system, run elevated (`sudo hmn ps`).
```

I called `ledger` and `proc_pidpath` directly from Python `ctypes`, sharing no code with `hmn`,
with and without a `sandbox-exec` profile that lets a process inspect only itself:

```
(version 1)(allow default)(deny process-info*)(allow process-info* (target self))
```

| PID | owner | unsandboxed `ledger` / `proc_pidpath` | sandboxed `ledger` / `proc_pidpath` |
|---|---|---|---|
| 393 WindowServer | `_windowserver` | rc 0 / 86 B | **`EPERM` / `EPERM`** |
| 1 launchd | root | rc 0 / 13 B | **`EPERM` / `EPERM`** |
| 2561 Safari | **hacker (same user)** | rc 0 / 90 B | **`EPERM` / `EPERM`** |
| self | hacker | rc 0 / 39 B | rc 0 / 39 B |

The "responsible process", which is the app macOS charges a privacy (TCC) request to, is the same
in both columns: PID 25217, the Claude Code app that spawned the shell. So nothing the parent app
was granted explains the difference. The only variable is the sandbox. A whole-system unsandboxed
scan agrees: `{(False, 0, 0): 249, (True, 0, 0): 670, (False, -1, 3): 1}`, keyed `(same user?,
rc, errno)`. That is 0 `EPERM` among 920 PIDs, and one cross-user process exited mid-scan with
`ESRCH`. A denial of only `process-info-ledger` gives `EPERM` from `ledger` even for the caller's
own PID.

This turns the doc claim around in both directions. Another user's process is readable
unsandboxed, and the caller's own processes are not readable sandboxed. Whether `sudo` helps a
sandboxed caller was not tested. A sandbox profile applies whatever the uid, so the advice to
re-run elevated is at best unproven.

The same claim, worded differently, appears in all of these places:

- `src/bin/hmn/main.rs:169-171` and `:194-196` ("run elevated (`sudo`) to include cross-user PIDs");
- `src/bin/hmn/ps.rs:505-509`;
- `src/gpu/metal.rs:5-11`, `:610-613` and `:682-684`;
- `src/gpu/mod.rs:374-377`.

## What `hmn` does under a sandbox — a silent zero

Here is the same binary under the profile above:

```
$ hmn ps
PID  NAME  VRAM  SHARED  DEVICE  SPILL
hmn: 0 GPU processes found.                                          # exit 0
$ hmn ps --filter safari --exit-status
hmn: 0 GPU processes found matching filter="safari".                 # exit 1
$ hmn watch 393 1 999999 99998 --interval 1s --duration 1s
hmn: watch failed to query device 0: no GPU measurement source available (NVML, DXGI, PDH, and nvidia-smi all failed or are disabled)   # exit 2
```

Unsandboxed, the same moment lists WindowServer at 264 MiB, Safari, WebKit and others.

- **`hmn ps` is wrong and says it is right.** `metal::list_compute_processes`
  (`src/gpu/metal.rs:619-708`) treats a failed `read_graphics_footprint` (`EPERM`, `ESRCH` or
  absent index) as "skip". Every row disappears and the result is `Some(vec![])`. The
  `N protected — re-run elevated for names` continuation counts only rows that were *listed*
  without a name, so it stays silent too.
- **`--exit-status` exits 1, which also means "no match".** A CI gate cannot tell "nothing
  matched" from "nothing was readable".
- **`hmn watch` blames the wrong cause.** It exits 2 with the `NoGpuSource` text. I did not trace
  which call fails first.
- With only `process-info-ledger` denied, so `proc_pidpath` still works, `watch` gets further. It
  shows WindowServer at `0 MiB` with no notice, because `process_exists` correctly says the PID
  exists and the row reads 0.

Who runs `hmn` sandboxed? A library consumer inside an App Sandbox app, or an agent harness that
wraps tool calls in a Seatbelt profile. Neither was tested here. The profile above is the minimal
shape of both.

A harsher profile that also denies `process-info*` on **self** crashes `hmn ps` with exit 133
(`SIGTRAP`). The fault is Apple's, not `hmn`'s: `libdispatch` aborts with `BUG IN LIBDISPATCH:
Unable to get the unique pid` inside `+[NSBundle mainBundle]`. Any Foundation or Metal program
dies there, and `hmn --version` alone runs fine.

## What check 3 actually exercised

`process_exists` is asked only about explicit PIDs absent from the first sample
(`src/bin/hmn/watch.rs:964-977`, called at `:1175`). Here is which branch each case took:

| PID | context | listed by `gpu_processes(0)` | `process_exists` asked? | answer |
|---|---|---|---|---|
| 393 WindowServer | unsandboxed | yes, 264 MiB | no | — |
| 1 launchd, 332 configd | unsandboxed | no (zero balance) | yes | `Some(true)`, no warning |
| 99998, 999999 | unsandboxed | no | yes | `Some(false)`, warns ✅ |
| any | sandboxed | `watch` exits 2 before asking | — | — |

The `None` arm (`src/gpu/metal.rs:710-731`) is correct and reachable: under the sandbox,
`proc_pidpath` gives `EPERM`. `hmn watch` never reaches it, because it fails earlier. Keep the
arm, and correct its doc example from "cross-user" to "sandboxed".

## What else is wrong — four smaller defects

**`--device 1` reports a missing backend instead of an out-of-range index.**

```
hmn: ps failed to query device 1: no GPU measurement source available (NVML, DXGI, PDH, and nvidia-smi all failed or are disabled)
```

Exit 2 and the prefix are as specified. The body names four backends that do not exist on macOS,
on a machine where device 0 works:

- `metal::list_compute_processes` returns `None` for any index other than 0 (`metal.rs:620`).
- That falls through to `bounds_check(device_index)?; Err(NoGpuSource)` (`src/gpu/mod.rs:456-457`).
- `bounds_check` (`src/gpu/mod.rs:604-624`) consults only NVML and DXGI.
- The public `device_count()` (`src/gpu/mod.rs:56-60`) also consults `metal::device_count` and
  gets 1.

So `DeviceIndexOutOfRange { index: 1, count: 1 }` is never produced on macOS. `watch --device 1`
prints the same text. `device_info`'s docs promise `OutOfRange` only for NVML/DXGI counts, so this
is a gap the docs allow, not a regression.

**`hmn watch 0` says kernel_task does not exist.**

```
hmn watch: device 0 [Apple M3 Pro], interval 1.0s, watching 1 PID(s)
hmn watch: pid=0 names no running process; its rows will read 0 MiB
```

`proc_pidpath(0)` returns `ESRCH`. This is the one false "no" observed, and `Some(false)` is the
answer the API promises never to give for a live process.

**SPILL and PAGED read `?` where spill does not exist.** The README says macOS `UMA` "has nothing
to spill *into*". That describes something that doesn't apply, not something unknown. The
crate already writes `n/a` for that kind of absence: the README capability table uses it for
reserved memory and driver version on Apple Silicon. `?`, meanwhile, carries two meanings in the
SPILL column, "doesn't apply" and "applies but unreadable now" (PDH hiccup, pre-`WDDM 2.0`), plus
a third in the NAME column ("unresolved name").

**The JSON summary says `"spilled":false` when it means "does not apply".** The summary line
begins `{"kind":"summary","measurable":false,"spilled":false,…`. The README and FAQ tell consumers
to check `measurable` first, but the field itself still says `false`. That is the collapse v0.2.13
removed from the text summary.

## Requests, in order of how much they would help

1. **Make an unreadable process list loud, not empty.** When `ledger` returns `EPERM` for every
   PID but the caller's own, or for most of them, `hmn ps` should say so. A possible shape:
   `hmn: 0 GPU processes found; 917 PIDs unreadable (EPERM — sandboxed caller?)`. It should not
   exit 0 as if the list were complete, and `--exit-status` should not return the "no match" code
   for it. `hmn watch` should report the same cause instead of `NoGpuSource`. **This is the change
   that matters most**: it is the one case where the instrument reports a wrong measurement as a
   correct one. The library side may need a count of `EPERM` skips next to the entries, in
   whatever shape fits the crate's API.
2. **Restate the macOS limitation from this evidence, at all seven sites listed above.** The
   readable set is decided by the caller's sandbox, not by process ownership. Unsandboxed, every
   user's processes are readable. Drop or qualify the `sudo` advice until someone tests whether
   it helps a sandboxed caller.
3. **Give `bounds_check` a Metal branch**, the same shape as the NVML and DXGI branches, fed by
   `metal::device_count()`. A unit test can pin it: on macOS, `gpu_processes(1)` returns
   `DeviceIndexOutOfRange`.
4. **Write `n/a` instead of `?` in SPILL and PAGED where the memory model has no separate pool**
   (macOS UMA), and keep `?` for "applies but unreadable now". Decide Linux on the same rule: it
   OOMs rather than pages, though managed-memory oversubscription exists. JSON can stay `null`,
   because `measurable` already carries the reason. **Make `spilled` null** (or absent) when
   `measurable` is false. That changes a persisted JSON contract, so it needs whatever
   compatibility note the crate gives such changes.
5. **Treat PID 0 as existing on macOS**, or skip the notice for it. kernel_task is always there.

## Smaller observations

1. Check 2's PID, 999999, is above the macOS PID max of 99999 (`ps -p 999999` →
   `process id too large`). It can never name a process, so the check is safe from PID reuse but
   does not test a realistic dead PID. 99998 does, and it passes too.
2. `tests/macos_smoke.rs` has two Metal tests that stay `#[ignore]` under `cargo test
   --all-features`: `device_info_reports_apple_brand` and `process_gpu_info_returns_metal_source`.
   They were not run here. `cargo test --test macos_smoke -- --ignored` covers them.
3. Column padding counts `char`s. That is correct for every name seen here; wide CJK or emoji
   process names would probably misalign. Not tested.
4. **Misreadings recorded.**
   - The first pass explained the `?`/0 MiB `watch` rows for launchd and configd as "the
     ledger/name lookups fail for them". The tool was right: the read succeeds with a zero
     balance, so the PID is unlisted and its row is built with name `?`.
   - The second pass concluded "no `EPERM` on this OS", which held only for an unsandboxed
     caller.

   The question that corrected it was whether the parent app's permissions were hiding a prompt.
   Answering it took the controlled sandbox comparison above.

## Acceptance fixtures (already run, free to regress against)

| Case | Command (uid 501, macOS 26.6.2, M3 Pro) | Expected | Observed |
|---|---|---|---|
| dead PID above PID max | `hmn watch 999999 <own PID> --interval 1s --duration 2s` | one warning, for 999999 | ✅ |
| dead PID in range | `hmn watch 99998 …` | warning | ✅ |
| cross-user, listed | `hmn watch 393` (WindowServer) | no warning, row from first sample | ✅ 264 MiB |
| root, zero balance | `hmn watch 1`, `hmn watch 332` | no warning | ✅ `?`/0 MiB |
| PID 0 | `hmn watch 0` | no warning | ❌ warns (request 5) |
| spill verdict | any `hmn watch` | `spill not measurable…` | ✅; cells `?` (request 4) |
| JSON summary | `hmn watch --json …` | `measurable:false`, `spilling_at_attach:null` | ✅; `spilled:false` (request 4) |
| out-of-range device | `hmn ps --device 1` | exit 2, `device index 1 out of range` | exit ✅, message ❌ (request 3) |
| `ledger`, unsandboxed | ctypes over all PIDs | per docs: `EPERM` for other users | rc 0 for 919/920, 1 `ESRCH`, 0 `EPERM` |
| `ledger`, sandboxed | ctypes, profile above | — | `EPERM` for 393, 1 **and same-user 2561**; rc 0 for self |
| sandboxed `ps` | `sandbox-exec -p '<profile>' hmn ps` | an unreadable-list notice, non-zero exit | ❌ `0 GPU processes found.`, exit 0 (request 1) |
| sandboxed `watch` | `sandbox-exec -p '<profile>' hmn watch 393 …` | the same notice | ❌ `NoGpuSource` text, exit 2 (request 1) |

## Confidence

**High** for the eight check verdicts:

- every command's stdout, stderr and exit code were captured separately and re-read by a second
  pass;
- the JSON was parsed, not eyeballed;
- the column alignment was computed, not judged.

**High** that the sandbox, not ownership, decides `EPERM`. The comparison is controlled: same
binary, same PIDs, same responsible app, sandbox on or off. A same-user PID flips to `EPERM` and
other-user PIDs read fine. The independent signal is direct `ctypes` syscalls that share no code
with `hmn`. `hmn ps`'s own unsandboxed listing of `_windowserver` corroborates them.

**Residual, reasoning rather than measurement:**

- The unsandboxed runs all had the Claude Code app as responsible process. The `ledger` and
  `proc_pidpath` checks are kernel MAC hooks, the Sandbox policy's, not TCC prompts, so a parent's
  TCC grants should not matter. A run from Terminal.app would close this gap.
- The probe's balance offsets were guessed. Only the zero-balance claims rest on them, and those
  are corroborated by `hmn ps` not listing the PIDs.
- Untested: `sudo` under a sandbox, a real App Sandbox (as opposed to `sandbox-exec`), and other
  macOS versions.

## References

- Issue: [mi-for-the-rust-of-us/hypomnesis#3](https://github.com/mi-for-the-rust-of-us/hypomnesis/issues/3)
  (the checklist this report answers)
- [dogfooding-spill-verdict-wording-and-ps-filters.md](dogfooding-spill-verdict-wording-and-ps-filters.md):
  the report v0.2.13 implements. Its "`?`, never `no`" rule is what request 4 refines into `n/a`
  vs `?`.
- `docs/roadmap-v0.2.13.md`; FAQ "What does a `?` in the NAME column mean"; README
  "macOS UMA semantics"
- Findings briefing and evidence: `__reports__/field_check_v0213/01-findings_v1.md` and
  `__reports__/field_check_v0213/evidence/`. The probes are `probes/p.py` (`proc_pidpath`),
  `probes/l.py` (`ledger` scan) and `probes/sandbox_probe.py` (sandbox on/off).
