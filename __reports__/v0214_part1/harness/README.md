# The v0.2.14 sandbox harness

One harness for the whole v0.2.14 campaign (issue #3): PR B (part 1) creates it, PR C
(part 2) extends it and never forks it. It runs `hmn` under named Seatbelt profiles,
captures normalised output that is safe to commit, and compares two captures.
`compare.py` is the byte-identity instrument for PR B and PR C: it is how either PR proves
that the output a sandbox policy already got right did not change.

| File | Role |
|:-----|:-----|
| `sandbox.sh` | `sandbox.sh [--print-profile] PROFILE [--] CMD [ARGS...]`: run `CMD` under a named profile, or print the profile text |
| `capture.py` | `python3 capture.py <hmn-binary> <outdir>`: six probes × the policies `none`, `P`, `C`, normalised |
| `compare.py` | `python3 compare.py <base> <new> [--spill-map]`: compare two capture directories for `none` and `C` |
| `codex.sb` | OpenAI Codex's Seatbelt base policy, byte for byte, plus a last line `(allow file-read*)` (Apache-2.0, see *The Codex pin*) |
| `CODEX_PIN` | the upstream source URL, commit, SHA-256 of `codex.sb` without its last line, and licence |

Requirements: macOS (`sandbox.sh` wraps `/usr/bin/sandbox-exec`), `bash` for `sandbox.sh`,
and Python 3 (standard library only; the system `/usr/bin/python3` 3.9 is enough) for
`capture.py` and `compare.py`. `capture.py` runs every probe through `sandbox.sh`.

## Rebuilding the two trees

Both captures use release binaries with default features. An `--all-features` build,
debug or release, enables the opt-in `debug-output` feature, whose `[nvidia-smi debug]`
lines on stderr would then be the first stderr line under `P`, `Q`, `S` and `S0`.

```bash
# The base: c1810a5, in its own worktree with its own target/ (never a shared CARGO_TARGET_DIR).
git worktree add --detach ../base-c1810a5 c1810a5
(cd ../base-c1810a5 && cargo build --release --locked)

# PR B: the tip of the PR branch.
cargo build --release --locked

H=__reports__/v0214_part1/harness
HMN_CAPTURE_RAW=$(mktemp -d) python3 $H/capture.py ../base-c1810a5/target/release/hmn __reports__/v0214_part1/c1810a5
HMN_CAPTURE_RAW=$(mktemp -d) python3 $H/capture.py "$PWD/target/release/hmn" __reports__/v0214_part1/pr_b
python3 $H/compare.py __reports__/v0214_part1/c1810a5 __reports__/v0214_part1/pr_b --spill-map
```

Run the shell gates under `bash`: in zsh, `cmd 2>&1 >/dev/null | ...` also carries stdout.

## Profiles

`sandbox.sh` exits with `CMD`'s exit code. An unknown option, an unknown profile
(`sandbox.sh: unknown profile '<x>'`) or a missing `CMD` exits `64`; it never falls back
to an unsandboxed run. `--print-profile` prints the text and exits `0` (for `C` the
contents of `codex.sb`, for `none` an empty line).

| Profile | Text, or how it runs | What it denies |
|:--------|:---------------------|:---------------|
| `none` | `exec CMD` | nothing |
| `P` | `(version 1)(allow default)(deny process-info*)(allow process-info* (target self))` | the report's profile: every `process-info*` call on another process, so `proc_listpids` too |
| `Q` | `P` + `(deny sysctl-read (sysctl-name-prefix "kern.proc"))` | `P`, plus the `kern.proc` sysctls |
| `S` | `P` + `(allow process-info* (target same-sandbox))`, run as `sandbox-exec -p "$text" bash -c '"$@"; rc=$?; exit $rc' _ CMD ...` | `P`, except on processes in the same sandbox; the `bash` wrapper stays resident, so `CMD` has a sibling |
| `S0` | the text of `S`, run directly | `CMD` is alone in its sandbox, so it behaves like `P` |
| `D` | `(version 1)(allow default)(deny process-info-pidinfo)(allow process-info-pidinfo (target self))` | `proc_pidpath` on another process; `proc_listpids`, `ledger` and `sysctl` are allowed |
| `L` | `(version 1)(allow default)(deny process-info-ledger)` | the per-process `ledger` read, for every process including the caller |
| `C` | `sandbox-exec -f codex.sb` | Codex's policy: `(deny default)`, `process-info*` allowed for `same-sandbox` |

The first stderr line of `hmn ps` under each profile, measured 2026-10-04 on the M3 Pro
(macOS 26.6.2), default-feature release builds, unsandboxed shell. The counts and sizes
follow the live process list; `D`'s rows include `hmn`'s own 16 KiB row, whose name it may
still read under `(target self)`, so its count is one more than its protected count.

| Profile | c1810a5 | PR B |
|:--------|:--------|:-----|
| `none` | `hmn: 19 GPU processes found (474 MiB committed total).` | `hmn: 19 GPU processes found (355 MiB committed total).` |
| `P` | `hmn: 0 GPU processes found.` (exit 0) | `hmn: ps failed to query device 0: no GPU measurement source available (Metal, NVML, and nvidia-smi all failed or are disabled) (skipped)` (exit 2) |
| `Q` | `hmn: 0 GPU processes found.` | as `P` |
| `S` | `hmn: 0 GPU processes found.` | as `P` |
| `S0` | `hmn: 0 GPU processes found.` | as `P` |
| `D` | `hmn: 19 GPU processes found (355 MiB committed total; 18 protected — re-run elevated for names).` | `hmn: 19 GPU processes found (355 MiB committed total; 18 protected — re-run outside the sandbox).` |
| `L` | `hmn: 0 GPU processes found.` (exit 0) | `hmn: 0 GPU processes found.` (exit 0: the residual PR C closes) |
| `C` | `hmn: 18 GPU processes found (355 MiB committed total).` | `hmn: 18 GPU processes found (355 MiB committed total).` |

## The Codex pin

R01 and R02 cite "OpenAI Codex's Seatbelt policy" and the recipe `sandbox-exec -f` with
Codex's `seatbelt_base_policy.sbpl` plus `(allow file-read*)`, but neither the file nor an
upstream commit was in the repository, so R01's Codex row could not be reproduced. The
file was downloaded with the maintainer's approval:

- source: `https://raw.githubusercontent.com/openai/codex/696b4502dfaacd91133c3691a23dc91ae3b14bc9/codex-rs/sandboxing/src/seatbelt_base_policy.sbpl`
- commit: `696b4502dfaacd91133c3691a23dc91ae3b14bc9`, the last commit to touch the file
  (unchanged at Codex's `main` when downloaded)
- size: 3910 bytes; SHA-256 `5103332ddb8885ee5e1926de6c0ef23a61f4e55e31297506ae05fa4e0b24ba74`

That hash is identical to the copy used while drafting the v0.2.14 plan, which had no
recorded commit, so R01's Codex row is reproducible from this pin. `codex.sb` is that file
plus one last line, `(allow file-read*)`. To check: `sed '$d' codex.sb | shasum -a 256`
prints the `sha256:` value of `CODEX_PIN`.

Licence: `codex.sb` is that file, redistributed under the Apache License 2.0
(<https://github.com/openai/codex/blob/696b4502dfaacd91133c3691a23dc91ae3b14bc9/LICENSE>),
with the attribution of Codex's NOTICE: "OpenAI Codex, Copyright 2025 OpenAI". The only
modification is the appended `(allow file-read*)` line. `CODEX_PIN`'s fourth line records
the same.

## Captures and their normalisation

`capture.py` runs six probes, `ps`, `ps_json` (`ps --json`), `ps_device_0`
(`ps --device 0`), `ps_pid_max_exit_status` (`ps --pid 4294967295 --exit-status`),
`ps_filter_windowserver` (`ps --filter WindowServer`) and `watch_0`
(`watch 0 --duration 2s --interval 1s`), under `none`, `P` and `C`, and writes
`<outdir>/<policy>/<probe>.{stdout,stderr,exit}`: 18 files of each kind per tree. The raw
output goes to `$HMN_CAPTURE_RAW` (default a fresh temporary directory); `capture.py` refuses
a raw directory inside the repository (exit `64`). The committed files are normalised, because the
maintainer's machine is public history:

- `[nvidia-smi debug]` lines are dropped, in `.stdout` and `.stderr`.
- The table probes (`ps`, `ps_device_0`, `ps_pid_max_exit_status`,
  `ps_filter_windowserver`): `.stdout` becomes a header `PID<TAB>NAME<TAB>DEVICE<TAB>SPILL`
  and one row per process, cut at the header's column starts. NAME becomes the first 8 hex
  digits of its SHA-256 (`?` stays `?`); VRAM and SHARED are dropped.
- `ps_json`: `.stdout` becomes `keys=<sorted key names>` (the union over the rows), or
  stays empty when `hmn` printed nothing (PR B under `P`). The row count is not kept: it is
  live data, and the table probes already compare the rows.
- Everything else (`watch_0`'s `.stdout`, every `.stderr`): sizes
  (`\d+(\.\d+)? (B|KiB|MiB|GiB)`) become `<size>`, `N GPU process(es) found` becomes
  `<N> GPU process(es) found`, `+N.Ns` times become `+<t>s`, runs of spaces collapse to
  one and trailing spaces are removed.
- `.exit` holds the exit code.

Where it is lossy:

- Hashed names prove a name did not change, not what it is; a short hash can collide.
- Processes come and go between runs, so a table is compared per PID over the PIDs present
  in both captures. A process that appears in one capture only is not compared, and that
  includes `hmn`'s own row, which has a new PID on every run.
- The column cut assumes ASCII names: `hmn` sizes columns in bytes and pads in characters
  (ROADMAP, "Text-table widths"), so a non-ASCII name shifts its row and the cut. None of
  the captured names is non-ASCII.
- `watch_0` is compared line by line, names included: it watches PID 0, whose name is `?`.
- `.stderr` keeps the `--filter` argument, `WindowServer`, as `hmn` echoes it.

The normalisation is stable for one binary: a second capture of the PR B binary, into a
directory outside the repository, compared with the committed one without `--spill-map`,
printed `identical` for both policies (2026-10-05: rows compared 47, spill cells mapped 0,
allowlisted lines 0).

## What "identical" means

`compare.py` compares the policies `none` and `C`, the two whose output already was right
and must not change. Per probe: `.exit` equal; `.stderr` equal after removing from `<base>`
the allowlisted lines; a table `.stdout` with an equal header and, per PID present in both,
an equal NAME hash, DEVICE and SPILL; every other `.stdout` equal line by line. "Identical"
is therefore: same exit codes, same stderr text up to sizes, counts and times, same names,
devices and SPILL cells for every process seen twice, same `watch` lines. `P` is not
compared: it changes by design in PR B (exit 0 to 2) and again in PR C, and each PR states
its own change in its findings.

The stderr allowlist (base side only), each removed line counted:

- a line containing `names no running process`: the `hmn watch 0` warning that PR B's
  `process_exists` fix removes.

`--spill-map` accepts a SPILL cell (table rows) or the last cell of a `watch_0` line (its
SPILL cell and its per-PID `PAGED` cell) that is `?` in `<base>` and `n/a` in `<new>`, and
counts it: PR B's `n/a` change. PR C compares PR B's captures with its own without it.

Output: one line per policy, `<policy>: identical after normalisation (rows compared: N,
spill cells mapped: M, allowlisted lines: K)`, N counting the table rows compared and M the
mapped cells, or `<policy>: DIFF` followed by each difference. Exit `0` when both are
identical, `1` on a difference, `2` on a usage error. Measured 2026-10-05, c1810a5 against
PR B with `--spill-map`: both policies `identical (rows compared: 47, spill cells mapped:
50, allowlisted lines: 1)`, exit 0; without `--spill-map`, exit 1; with one NAME hash
edited in a scratch copy of `pr_b/none/ps.stdout`, exit 1.

## Extension points for PR C

- `sandbox.sh`: PR C adds a `--job` option (a resident GPU job inside the sandbox) and its
  fixtures (`gpujob.swift`, `count_denied.py`) beside this file. In PR B any other option
  exits 64, so `--job` is an addition to this file, not a second script.
- `compare.py`: PR C runs it against these captures without `--spill-map`, so PR B's `n/a`
  cells are its base. An allowlist entry is added to `STDERR_ALLOWLIST` with its reason,
  and named here, never masked silently.
- `capture.py`: a new probe is a new entry in `capture.py`'s `PROBES` (and in
  `TABLE_PROBES` if it prints the `ps` table) and in `compare.py`'s `PROBES`, and a line in
  this README.
