# kinfo_enumeration: what each sandbox profile shows

The Metal backend enumerates with `sysctl(KERN_PROC_ALL)` when `proc_listpids` is refused, names a denied process from `p_comm` only where `proc_pidpath` is refused, and reads each per-PID ledger as bytes, denied, gone or unavailable. This record is the measured before and after of what the leaf can judge from the command line, run through the one harness in `__reports__/v0214_part1/harness/`.

Host: Apple M3 Pro, macOS 26.6.2, arm64, stable Rust 1.99.0. Binaries: `cargo build --release --locked` (default features) of the tree after Steps 1 to 3 (after) and of b716087 (before: PR B merged, built in a detached worktree under `target/base`, removed afterwards). `H` is the `hmn` binary of the tree, `K` is `__reports__/v0214_part1/harness`. Every command ran as `bash $K/sandbox.sh PROFILE [--job] -- CMD`, from the repository root. Counts of processes follow the live process list, so they differ from run to run; the shape of each result does not. No process list is reproduced here: only counts and exit codes.

## Changes in the harness

| File | What it is |
|:-----|:-----------|
| `sandbox.sh` | `sandbox.sh [--print-profile] PROFILE [--job] [--] CMD`. `--job` is read after the profile check: `Z` (unknown) and `Z --job` exit 64, and `--job S` (before `PROFILE`) exits 64. With `--job`, `gpujob` is compiled into `target/gpujob` outside the sandbox, started inside it, and its PID is `$JOB` for `CMD`; a `trap` kills the job when the script exits for any reason |
| `gpujob.swift` | a resident 256 MiB `.storageModeShared` Metal buffer, written in full |
| `count_denied.py` | an independent probe: `sysctl` MIB `{1, 14, 0}`, `ledger` command 4 per PID, prints `denied=<n> read=<m> gone=<k>`; `EPERM` is denied, `ESRCH` is gone, its own PID is never counted; exit 1 unless the listing is whole 648-byte records and holds its own PID |
| `compare_names.py` | two raw `ps --json` files: for each PID in both whose first name is not `null`, the second name equals the first or equals it cut at 16 bytes and back to a whole character; a `null` second name and any other prefix are mismatches; exit 1 when nothing was compared |

## Profile S, with a resident job (`--job`)

`$JOB` is the 256 MiB buffer started inside S's bash wrapper, a sibling of `hmn`.

| Command under `sandbox.sh S --job --` | b716087 | after |
|:--|:--|:--|
| `hmn ps --json --pid $JOB` | exit 2, empty stdout, `(skipped)` line, then `hmn: ps: no device could be queried, so nothing could be listed`; no `"name":"gpujob"` row | exit 0, one row `"name":"gpujob"`, `"used_bytes":268451840`; stderr `hmn: 1 GPU process found matching pid=<JOB> (256 MiB committed total).` |
| `hmn ps --json --pid $JOB`, profile `none` | not run | the same row, `"used_bytes":268451840` |
| `hmn watch $JOB --duration 2s --interval 1s` | not run | exit 0; two sample rows and the closing per-PID row, each naming `gpujob` at 256 MiB; `hmn watch: device 0 [Apple M3 Pro], interval 1.0s, watching 1 PID(s)` |

At c1810a5 and 40a701e the leaf records, from its own measurement, `exit 0`, stdout `[]` and `hmn: 0 GPU processes found matching pid=<JOB>.`: not re-measured here.

## `ps --device 0` under each profile

Each cell is `<exit> <lines with "no GPU measurement source"> <lines with a nonzero "N GPU process" summary>`.

| Profile | b716087 | after | Why |
|:--|:--|:--|:--|
| S | `2 1 0` | `0 0 1` | the caller reads itself (16 KiB) and the sibling's zero balance makes `others_read` at least 1, so the list is `Ok`: `1 GPU process found` |
| S0 | `2 1 0` | `2 1 0` | alone in its sandbox: every other read is denied, `others_read` is 0, the bridge returns `None` |
| Q | `2 1 0` | `2 1 0` | `kern.proc` denied: both enumerations refused |
| L | `0 0 0` (`0 GPU processes found matching device=0.`) | `2 1 0` | the caller's own read is denied too and every other read is denied: `None` |

Under L the stderr is `hmn: ps failed to query device 0: no GPU measurement source available (Metal, NVML, and nvidia-smi all failed or are disabled)`, the bridge's text; `gpu_process_listing` owns the `process list unreadable:` text.

Plain `hmn ps` (no `--device`), first stderr line:

| Profile | b716087 | after |
|:--|:--|:--|
| S | exit 2, the `(skipped)` line | exit 0, `hmn: 1 GPU process found (0 MiB committed total).`, one table row (`hmn` itself) |
| S0, P, Q | exit 2, the `(skipped)` line | the same |
| L | exit 0, `hmn: N GPU processes found.` | exit 2, the `(skipped)` line |

Under S, plain `hmn ps` lists `hmn`'s own row and exits 0 although nearly every other process was denied: `others_read` is 1 and the denied PIDs are not counted in the summary. The summary and the remedy are `part2/api/cli/ps_watch_unreadable`'s; this leaf does not change them.

## Profile P

`ps --device 0`: exit 2, stdout empty, one stderr line ending `(Metal, NVML, and nvidia-smi all failed or are disabled)`. `ps`: exit 2, that line plus the `(skipped)` line, stdout empty. Identical to b716087: the bridge turns the all-denied list (hmn's own 16 KiB row, everything else denied) into `None`, so `NoGpuSource` and exit 2 hold and the silent zero does not return.

## Profile D (`process-info-pidinfo` denied except on self)

`proc_listpids` and `ledger` are allowed; only `proc_pidpath` is refused, so every name comes from the `KERN_PROC_PID` `p_comm` lookup.

| | b716087 | after |
|:--|:--|:--|
| rows in `ps --json` | 31 | 31 |
| rows with `"name":null` | 30 | 0 |

`compare_names.py` against an unsandboxed `ps --json` captured in the same minute: `compared 30 mismatch 0`, exit 0. A row whose name the kernel cut (for example a 20-byte name shown as its first 16 bytes) equals the unsandboxed name cut at 16 bytes. On two captures whose names are all `null`, `compare_names.py` prints `compared 0 mismatch 0` and exits 1; a name beside `""`, beside a 15-byte cut or beside `null` is a mismatch.

## `count_denied.py`

| Profile | Output |
|:--|:--|
| none | `denied=0 read=1062 gone=0` |
| S | `denied=1062 read=1 gone=0` |
| P | `denied=1062 read=0 gone=0` |
| S0 | `denied=1062 read=0 gone=0` |

The one read under S is the bash wrapper, the probe's sibling. With the listing replaced by an empty one, or by one without the probe's own PID, the probe exits 1 and prints no counts.

## Captures and the byte comparison

`capture.py` ran on the b716087 release build, then on this tree's, in the same boot, one right after the other: `__reports__/v0214_part2/captures/kinfo_enumeration_base/` and `__reports__/v0214_part2/captures/kinfo_enumeration/` (policies `none`, `P` and `C`, six probes, 54 files each). `pr_b/` is not the base: `compare.py` matches rows by PID and a reboot since it was captured reassigns them.

```
python3 $K/compare.py <base> <new>
none: identical after normalisation (rows compared: 61, spill cells mapped: 0, allowlisted lines: 0)
C: identical after normalisation (rows compared: 61, spill cells mapped: 0, allowlisted lines: 0)
rc=0
```

Where libproc works (`none`, `C`) the output did not change. `P` is not compared: it stays exit 2 with the same text on both sides.

## Harness interface, unchanged by `--job`

`sandbox.sh Z -- /usr/bin/true` exits 64; `sandbox.sh Z --job -- /usr/bin/true` exits 64; `sandbox.sh --job S -- /usr/bin/true` exits 64; `sandbox.sh --print-profile P` prints the one-line profile text.

## Residuals

- A refused and a failed `KERN_PROC_ALL` both end as "can't enumerate" (`None`) downstream; `classify_kinfo_all` separates `ENOMEM` (retry) from every other failure, and no outcome of `hmn` shows a difference between the rest.
- A kernel probe so large that the buffer-size arithmetic saturates is not handled beyond `vec!`'s own allocation failure; the measured probe is under 700 KB.
- `proc_listpids` and `proc_pidpath` can in theory return 0 without setting `errno`, so `errno` may be stale. The worst outcome is a `p_comm` name or a `sysctl` fallback, never a denial.
- Wiring that neither the unit tests nor the CLI gates reach, pinned by reading and review only: the `rc`, `errno` and `len` handed from the `KERN_PROC_ALL` fill to `classify_kinfo_all`; the 1/8 slack; the bound of four attempts; the libproc-first order in `list_pids`; the probe's `rc` check; `namelen` 4, which the kernel still accepts; the `errno` read in phase 2 of `list_libproc_pids`; an unresolved ledger index read as entry 0; the `idx < 0` path issuing no syscall; and `kern_proc_pid_comm` handing its classifier `rc` 0.
- `count_denied.py` prints a passing line for a listing that holds only its own PID, and the `sandbox.sh --job` trap is checked by hand only.
- Not reachable from a test or from this host, and so pinned by reading and by the harness gates: the `ENOMEM` retry against a real kernel (the classifier is unit-tested with literal answers; the four-attempt loop is inline), and the wiring of `trust_kinfo_listing` (a grep pin).

## Expected, not re-measured

c1810a5 and 40a701e, from the leaf: S exit 0 with `hmn: 0 GPU processes found`; D 12 rows with 11 nameless; P exit 0 plain and exit 2 with `--device 0`; L exit 0.
