# `hypomnesis` v0.2.14 — roadmap

> *Measure what a sandbox allows, say what it forbids, and stop reporting an unreadable list as
> an empty one.*

**Status: in progress.** Not pushed; no version bump until release.

---

## Why v0.2.14 (and not v0.3.0)

Every item is additive under the crate's own rule (`ROADMAP.md`, Principle 2: *"New variants and
fields land in patch releases. Type-shape changes … are minor bumps, never patches."*):

- one new public function, `hypomnesis::gpu_process_listing`, and its `#[non_exhaustive]`
  result struct; `gpu_processes` keeps its signature and becomes a thin wrapper;
- one new `HypomnesisError` variant on the `#[non_exhaustive]` enum, as `Pdh` once was;
- a Metal arm in the private `bounds_check`, and macOS-only `sysctl` fallbacks inside the private
  Metal backend;
- wording changes to human-readable output: `n/a` instead of `?` in the SPILL and `PAGED` cells
  where spill cannot exist, an `unreadable` part on the `hmn ps` summary line, and a
  platform-correct remedy in place of "re-run elevated" on macOS.

Three behaviour changes are deliberate. Each turns a silent wrong answer into a stated one:

- `gpu_processes` returns an error, not an empty list, when the process list was enumerated but
  no process other than the caller's could be read;
- `hmn ps` exits `2` when every device it tried failed, where it now prints an empty table and
  exits `0`;
- `hmn ps --exit-status` exits `2` ("can't tell") rather than `1` ("nothing matched") when nothing
  is listed and some processes could not be read.

One request is **not** in this release: making the JSON `spilled` field `null` when spill is not
measurable. It changes a `bool` into a `bool` or `null` on the wire, a type-shape change, so it
waits for v0.3.0 (see *Design decisions*).

---

## Origin — a field check on Apple Silicon, and what checking it found

[`docs/dogfooding-feedbacks/dogfooding-macos-sandbox-eperm-and-device-bounds.md`](dogfooding-feedbacks/dogfooding-macos-sandbox-eperm-and-device-bounds.md)
(the issue #3 field check, 2026-10-01, Apple M3 Pro, macOS 26.6.2). All eight checks of
[mi-for-the-rust-of-us/hypomnesis#3](https://github.com/mi-for-the-rust-of-us/hypomnesis/issues/3)
passed as worded. The premise of check 3 did not. The crate says that macOS cross-user PIDs are
skipped because `ledger` returns `EPERM`, and that `sudo hmn ps` lists them. Measured, ownership
plays no part:

- **Unsandboxed:** other users' processes, root's included, read fine (0 `EPERM` over 920 PIDs).
- **Sandboxed:** under a sandbox that denies `process-info*`, even the caller's own Safari gets
  `EPERM`, and `hmn ps` prints `0 GPU processes found.` with exit `0`.

Requests, in the report's order:

1. make an unreadable list loud;
2. restate the macOS limitation;
3. give `bounds_check` a Metal arm;
4. `n/a` rather than `?` where spill does not exist, and a `null` `spilled`;
5. PID 0 on macOS.

Checking the report against the code and the kernel confirmed it, and went further in five places.

- **The cause is in the kernel source.** XNU's `ledger()` (`bsd/kern/sys_generic.c`) returns
  `ESRCH` from `proc_find`, then consults only `mac_proc_check_ledger`, the sandbox's hook. There
  is no uid check. The crate's own first macOS probe (May 2026, `__reports__/macos_ledger/00-findings_v0.md`
  in commit `7045b5c`) had already read WindowServer's ledger unprivileged. "Cross-user needs
  root" came from `task_for_pid` and was never measured for `ledger`.
- **The report's mechanism for the silent zero is wrong for its own profile.** Under
  `(deny process-info*)(allow process-info* (target self))` it is **`proc_listpids`** that fails,
  not the per-PID reads. `list_compute_processes` returns `None`, so `gpu_processes(0)` returns
  `NoGpuSource`, whose text names four backends macOS doesn't have. `hmn ps --device 0` then
  exits `2`. Plain `hmn ps` exits `0` with an empty table, because `run_ps` drops a failing device
  without a word (`src/bin/hmn/ps.rs:403`), **on every platform**. The per-PID silent skip the
  report describes is real, but only under a profile that denies `process-info-ledger` alone.
- **A real App Sandbox behaves the same.** Built with an embedded `Info.plist` and ad-hoc signed
  with `com.apple.security.app-sandbox`, `hmn ps` printed `0 found` (exit `0`), `--device 0`
  exited `2` and `hmn watch` exited `2` with the `NoGpuSource` text.
- **The doc claim sits at about sixteen sites, not seven.** The README capability table even says
  "libSystem syscalls always succeed", and `ROADMAP.md` Principle 4 cites "macOS cross-user
  `EPERM`".
- **The sandbox still lets through more than `hmn` uses.** `sysctl(KERN_PROC…)`, the call `ps(1)`
  is built on, enumerates every process, names it and tells a live PID from a dead one under the
  report's profile. `ledger` reads of the caller's own sandbox (an agent's training job) succeed.
  `hmn` measures neither today, because it stops at the refused `proc_listpids`.

**When it bites.** Every earlier macOS test and benchmark ran in an unsandboxed shell, where nothing
is refused. A `(deny default)` profile does not refuse `process-info` either. Under OpenAI Codex's
Seatbelt policy, which allows `process-info*` only for `same-sandbox` targets, `hmn ps` lists all
20 processes correctly; Chromium's `common.sb` keeps a TODO to deny it explicitly. It takes an
explicit `(deny process-info…)`, or the App Sandbox:

| Caller | `proc_listpids` | others' `ledger` | `proc_pidpath` | `sysctl kern.proc` | `hmn` 0.2.13 |
|---|---|---|---|---|---|
| unsandboxed | ok | ok | ok | ok | correct |
| Codex Seatbelt policy | ok | ok | ok | `kern.proc.all` denied, `kern.proc.pid` ok | correct |
| App Sandbox | `EPERM` | `EPERM` (self ok) | `EPERM` | not yet measured in-app | `0 found`, exit `0` |
| explicit `deny process-info*` (the report's profile; agent sandboxes that deny it to stop argv leaks) | `EPERM` | `EPERM` | `EPERM` | ok (969 processes) | `0 found`, exit `0` |
| the same, with `same-sandbox` allowed | `EPERM` | ok for the sandbox's own jobs | ok for them | ok | `0 found`, exit `0`, though the job is readable |
| `process-info-pidinfo` denied outside the sandbox (`agent-safehouse` v0.12) | ok | ok | `EPERM` | ok | right numbers, names `?`, "re-run elevated" |

**Why Windows never showed it.** PDH on Windows, and NVML on Linux, return every process's VRAM
from one system-wide query, with no permission check per process. Only names can be refused there,
and v0.2.8's `Toolhelp32` fallback and `[protected]` bracket handle that. macOS is the one platform
that reads each PID separately, so it is the one where some rows can go missing.

---

## Design decisions taken before starting

- **Measure everything the sandbox permits, with one lookup rule.** On macOS every process
  lookup asks libproc first, and asks `sysctl kern.proc` only when libproc refuses or answers
  "no". The rule covers:
  - enumeration: `proc_listpids`, then `KERN_PROC_ALL`;
  - names: `proc_pidpath`, then `p_comm`;
  - existence: `proc_pidpath`, then `KERN_PROC_PID`.

  Neither source alone covers every caller: Codex's policy refuses `kern.proc.all` and allows
  libproc, while an explicit deny does the reverse. Because libproc answers first, every case that
  works today, unsandboxed and under Codex, keeps byte-identical output.

  One private helper reads `kinfo_proc` records as `[u8; 648]` with named offsets. It needs no
  `libc` dependency, checks that the length is a whole number of records, and takes `p_comm` from
  the record, so the enumeration fallback gets names in the same pass. Record parsing and errno
  classification are pure functions with unit tests, the way `proc_name.rs` tests its own; the
  sandbox paths cannot be unit-tested any other way.
- **A per-PID read has four outcomes, not two.** `read_graphics_footprint` stops folding
  everything into `None`. It returns bytes; *denied* (`EPERM`); *gone* (`ESRCH`); or
  *unavailable*, when the `graphics_footprint` template index did not resolve. *Unavailable*, or
  both enumerations refused, makes the backend return `None`. The dispatcher then falls through
  to `NoGpuSource`, as for every other backend, instead of today's silent empty list.
- **Names follow the Linux rule literally.** The `p_comm` fallback is used only on `EPERM`, never
  on `ESRCH`, so a PID's name cannot flip between sources and trigger `hmn watch`'s PID-reuse reset
  (`src/bin/hmn/watch.rs:316-327`). A `p_comm` shorter than 16 bytes is exact; one of 16 may be cut
  and is returned as is. That is what `proc_name.rs` does with Linux's 15-byte `comm` when nothing
  extends it. The `GpuProcessEntry::name` docs, the FAQ and the `--filter` help each say so in one
  sentence: a name `--filter` cannot match past the cut.
- **`process_exists` follows the same rule**, which also fixes PID 0:
  - `proc_pidpath` gives a path → `Some(true)`;
  - otherwise `KERN_PROC_PID` decides: a record → `Some(true)`, no record → `Some(false)`;
  - if that call is refused too: `ESRCH` → `Some(false)`, anything else → `None`.

  kernel_task has no executable path, so `proc_pidpath(0)` says `ESRCH`, but `KERN_PROC_PID`
  finds it, with no special case. A sandboxed caller also gets an answer where it used to get
  `None`.
- **State what is forbidden, by count, with one dispatcher.**
  - `gpu_process_listing(device_index) -> Result<GpuProcessListing>` is the one dispatcher; it
    carries `entries` and `denied_pids: Vec<u32>`. `gpu_processes` maps it to its entries.
  - `denied_pids` is a list, not a count, so `hmn ps --pid` and `hmn watch` can ask about a given
    PID. It is always empty on Linux and Windows, as a per-platform doc table says.
  - `HypomnesisError::ProcessListDenied { denied: u32 }` is returned only when the list was
    enumerated but no process other than the caller's could be read, counting zero-balance reads
    as read. Its `Display` describes what happened, like its siblings.
  - The remedy lives in one CLI constant, not in `Display`, so there are never two phrasings to
    drift apart.
  - No `test-helpers` builder is added: nothing outside the crate builds a listing, and the
    formatters take slices.
- **The remedy copies the Windows one.** Since v0.2.2 the `hmn ps` summary line has ended with
  `(N protected — re-run elevated for names)`. Since v0.2.6 `hmn watch` has said
  `re-run elevated to identify`. macOS gets the same shape, `N unreadable — re-run outside the
  sandbox`, in the same parenthetical and positions.

  | | Windows | macOS |
  |---|---|---|
  | What is withheld | names only; "measurement itself never needs elevation" (FAQ) | the bytes of processes outside the caller's sandbox |
  | Remedy | re-run elevated | re-run outside the sandbox: an agent harness's escalation or unsandboxed mode; there is none inside an App Sandbox app |

  The remedy is one `cfg!`-selected constant in `format.rs`. The summary, the skipped-device line,
  `watch`'s growth hint and its notices all use it. On macOS, when both counts are non-zero, it is
  said once. Windows and Linux output stays byte-identical. No `sudo` advice remains for macOS:
  unsandboxed it is never needed, and inside a sandbox it is not known to help.
- **`hmn ps` states a skipped device, and fails when every device failed.** Skipping a failing
  device without `--device` stays, as v0.2.13 decided, so one broken device does not hide the
  others. It now gets a stderr line. When every device tried has failed, `hmn ps` exits `2`. When
  no device was tried at all (`device_count` failing on a GPU-less runner), it keeps today's empty
  table and exit `0`.
- **`hmn ps` and `hmn watch` report a partial denial.**
  - `ps`: `SummaryNotes` gains `unreadable`. `--pid` applies to denied PIDs the way `judge`
    applies it to rows (`ps.rs:260`), so `hmn ps --pid N` does not report hundreds of unrelated
    unreadable processes. `--exit-status` exits `2` when nothing is listed and a relevant PID was
    denied.
  - `watch`: one notice per denied explicit PID ("unreadable here; its rows will read 0 MiB"),
    and denied PIDs are kept out of `missing_pid_notices`. The "found no GPU processes" message
    (`watch.rs:1119-1124`) says why when processes were denied. A sandbox does not change
    mid-run, so notices come once at attach; `--follow-new` gets a count; the per-interval error
    path does not repeat the remedy.
  - JSON stdout keeps its shape; the notices go to stderr, as v0.2.10 did for the no-subcommand
    path.
- **`n/a` where spill cannot exist, `?` where it can't be read now.**
  - `n/a` on Linux and macOS, decided at compile time: the README capability table already writes
    `n/a` there.
  - `?` stays on Windows: a PDH hiccup, pre-`WDDM 2.0`, or a build without `pdh`.
  - `is_spill_measurable()` is not used: it is a runtime PDH probe that also folds in pre-`WDDM 2.0`,
    which is the `?` case.
  - A pure core takes the platform answer as a parameter, so the tests run on any OS.
    `format::spill_cell` wraps it, and `hmn watch`'s per-PID `PAGED` renderer (`watch.rs:521-525`)
    shares it, so the two surfaces cannot diverge.
  - JSON stays `null`. The FAQ adds that CUDA managed-memory oversubscription is not measured
    either.
- **`spilled: null` waits for v0.3.0.** `write_spill_report_fields` feeds both `hmn spill --json`
  and `hmn watch --json`; turning `false` into `null` is a wire type change. It is written up as
  [`__reports__/field_check_v0213/02-notice_spilled_null_v0.md`](../__reports__/field_check_v0213/02-notice_spilled_null_v0.md)
  and logged under `ROADMAP.md` "Speculative: v0.3.0". The FAQ's "check `measurable` first" stays.
- **The doc fix is one canonical statement.** The README Limitations bullet says the sandbox
  decides; that unsandboxed, every user's processes are listed; and that inside, `hmn` measures
  what is permitted and counts the rest. Every other site says it in one line and points there,
  so no site restates the mechanism and drifts again. One FAQ line covers the self-denying profile
  that crashes any Foundation program inside Apple's `libdispatch`, before `hmn` runs.
- **Logged, not built:**
  - naming the GPU clients a sandbox hides: IORegistry `AGXDeviceUserClient` survives the sandbox
    with pid, name and GPU time but no bytes, and needs an IOKit binding;
  - display width for CJK and emoji names: column widths are bytes and padding is chars, which
    holds wherever one char is one column, and the fix needs East Asian Width data.

  Both go to `ROADMAP.md`.

---

## Scope

| # | Item | Kind | Status |
|---|---|---|---|
| 1 | Metal arm in `bounds_check`; Metal named in `NoGpuSource` and the `# Errors` docs (request 3) | **fix** | ⬜ |
| 2 | `process_exists` through the lookup rule: PID 0, sandboxed callers (request 5) | **fix** | ⬜ |
| 3 | `sysctl kern.proc` enumeration and `p_comm` names when libproc is refused; four-outcome ledger read (request 1) | feature | ⬜ |
| 4 | `gpu_process_listing`, `denied_pids`, `ProcessListDenied` (request 1) | feature | ⬜ |
| 5 | `hmn ps` states a skipped device and exits `2` when every device failed (request 1) | **fix** | ⬜ |
| 6 | `unreadable` counts, the platform remedy, `--exit-status` `2`, `watch` notices (request 1) | feature | ⬜ |
| 7 | `n/a` vs `?` in SPILL and `PAGED` cells (request 4, cells) | fix | ⬜ |
| 8 | The macOS limitation restated from evidence, one canonical statement (request 2) | docs | ⬜ |
| 9 | `spilled: null` notice and `ROADMAP.md` v0.3.0 entries (request 4, JSON) | docs | ✅ notice written |
| 10 | Correct the field report's F5 mechanism and site list, before the issue comment | docs | ⬜ |
| 11 | README, FAQ, tutorials, `CHANGELOG.md`, `ROADMAP.md` | docs | ⬜ |

Items 3, 4 and 6 get an adversarial review before they merge. They change what the instrument
reports and the exit codes scripts gate on. Every item updates the `CHANGELOG.md`, `--help`,
README and FAQ text it makes stale, as in v0.2.13. Item 11 covers what remains.

---

## Verification

To be filled in as items land. These fixtures must be re-run:

- every row of the *When it bites* table, with `sandbox-exec` and the same profiles:
  - the report's profile: exit `2` with the count and the remedy;
  - `same-sandbox` allowed: the job listed **with its bytes**, plus the unreadable count;
  - pidinfo denied: names, not `?`;
  - unsandboxed and the Codex policy: output unchanged;
- the App Sandbox build, including `KERN_PROC_ALL` inside it;
- `hmn ps --device 1` → `device index 1 out of range (have 1 devices)`;
- `hmn watch 0` → no warning;
- `cargo test --test macos_smoke -- --ignored` (2/2 at `cf5ada0`).

Gate set on every commit:

- `cargo fmt --check`;
- clippy with and without `--all-features`, and for `x86_64-unknown-linux-gnu`;
- `cargo test --locked --all-features`;
- `cargo doc` with `-D warnings`.

`tests/cli_ps.rs` accepts exit `2` only together with the denial line, since agents run
`cargo test` inside sandboxes too. Future field checklists use a realistic dead PID: macOS PIDs
stop at 99999, and on Linux `pid_max` can exceed 999999.

---

## Consistency pass

To be run with fresh eyes after the last commit.

---

## At release

- `Cargo.toml` bumped to `0.2.14`; this roadmap's status and the dogfooding report's `Status`
  flipped, per the dogfooding style guide.
- The README's "what's new" banner rotated: 🆕 `0.2.14`, `0.2.13` to 🚀, `0.2.11` dropped.
- `__reports__/` dropped from the tree before the merge, as in `f3c6010`; it stays in history.

---

## References

- [`docs/dogfooding-feedbacks/dogfooding-macos-sandbox-eperm-and-device-bounds.md`](dogfooding-feedbacks/dogfooding-macos-sandbox-eperm-and-device-bounds.md)
  — the report this release implements.
- [`__reports__/field_check_v0213/`](../__reports__/field_check_v0213/) — the findings, evidence,
  probes and the `spilled` notice.
- [`docs/roadmap-v0.2.13.md`](roadmap-v0.2.13.md) — `process_exists`, `--exit-status`, and the
  decision to skip a failing device when `--device` is not given.
- [`docs/roadmap-v0.2.10.md`](roadmap-v0.2.10.md) — the silent-`[]` fix whose shape (a stderr
  statement, an unchanged JSON shape) the denial counts follow.
- XNU `bsd/kern/sys_generic.c`, `ledger()`; Apple DTS on libproc in the App Sandbox
  ([691857](https://developer.apple.com/forums/thread/691857),
  [52941](https://developer.apple.com/forums/thread/52941)); OpenAI Codex
  `codex-rs/sandboxing/src/seatbelt_base_policy.sbpl`.
