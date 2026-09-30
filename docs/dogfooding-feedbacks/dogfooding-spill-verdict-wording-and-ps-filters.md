# Dogfooding report (from askesis canvas): on Linux `watch --filter` finds the trainer at once but its summary says "no spill observed" unmeasured; on Windows `ps` caught a real spill — the trainer's own, not the desktop's — and flags every row on the card with it

**Date:** 2026-09-28
**Reporter:** askesis `canvas` trainer (candle 6L/384d MDLM, batch 128, 16.5 GiB per training process). Campaign 1: run `acspv_pl_s1_d1` on a rented vast.ai box — RTX 5090 32 GiB, driver 580.95.05, AMD Ryzen 7 7800X3D, Linux container (NVML backend), `hmn 0.2.12` from `cargo install hypomnesis`. Campaign 2: a one-step check of the same trainer on the author's desktop — RTX 5060 Ti 16 GiB, driver 610.88, Windows 11 / WDDM (PDH backend), `hmn 0.2.12`
**Severity:** Field validation of v0.2.12 `hmn watch --filter` / `--min` / `start` record on Linux, and of `hmn ps`'s SPILL verdict on a real Windows spill (both right) + one wording defect + requests for **v0.2.13**
**Affected area:** `hmn watch` text summary on non-measurable platforms; `hmn ps` SPILL column attribution (per-device broadcast) and summary line; `hmn ps` selection (`--pid`, `--device`) and exit status; `hmn watch` with an explicit PID that does not exist
**Status:** ✅ **Resolved in v0.2.13** (2026-09-30). All six requests and the three smaller observations shipped: `hmn watch` says `spill not measurable on this platform` (request 1, a bug since v0.2.6); `hmn ps` marks the paged process `PAGED` / `device`, adds `paged` and `shared_share` to `--json` and states the device verdict once (request 2, as *paged*, not *cause*); `ps --filter`, `--exit-status`, `--device` range errors and a repeatable `--pid` (requests 3–6); a warning for a nonexistent explicit PID via a new `process_exists`, without guessing (observation 1); aligned `watch` columns (observation 2); `--help` commands before Limitations (observation 3). Validating it found two more, also shipped: `hmn watch` now marks `PAGED` too, and says when it attached to a spill already under way — which its growth-based verdict cannot count (`ROADMAP.md`'s `spill_condition` item, un-gated by this report).

---

> **Revised 2026-09-30** after checking the report against the numbers it quotes and the crate's
> own documentation. The first version blamed the Windows spill on the desktop beside the trainer,
> treated killing the process as proof of spill, and framed request 2 as naming a spill's *cause*.
> All three were wrong or overstated. The trainer does not fit on a 16 GiB card on its own. The
> timings corroborate the spill but do not prove it. Shared residency shows which process is being
> paged, not which one caused the pressure. Smaller fixes: the Campaign 2 output block was two
> commands, now labelled apart; "agreeing to the MiB" is now "within display rounding";
> observations 1 and 2 are narrowed. The Linux finding and every measured value are unchanged.

## TL;DR

On Linux, v0.2.12's `hmn watch --filter canvas` picked the one trainer out of the machine first
time, and `hmn` agreed with `nvidia-smi` on its footprint (16.5 GiB against 16,912 MiB). On
Windows, `hmn ps` flagged a real spill: the same trainer at **14.2 GiB dedicated + 1.9 GiB
shared** on a 16 GiB card. The trainer alone exceeds the card, so it spilled whatever else was
running, and its step was slower than the same step on the CPU.
Three findings. **(1)** On Linux, `watch`'s text summary says `episodes 0 — no spill observed`
for a run it could not measure (`"measurable": false` in the JSON). This matters most.
**(2)** On Windows, SPILL is marked on every row of the device, `hmn.exe` at 0 MiB included.
**(3)** `hmn ps` selects only by PID, and a launcher holds the wrapper's PID.

---

## What worked — `watch --filter` selects the workload on Linux, first time

```
$ hmn watch --filter canvas --interval 10s --duration 25s
TIME      PID     NAME          COMMITTED  ΔCOMMIT    SHARED     ΔSHARED    SPILL
+0.0s     15534  canvas  16.5 GiB   +0 B      0 MiB   +0 B      ?
hmn watch: device 0 [NVIDIA GeForce RTX 5090], interval 10.0s, watching 1 PID(s) (top 5 by committed among names containing "canvas" (case-insensitive))
+10.0s    15534  canvas  16.5 GiB   +0 B      0 MiB   +0 B      ?
+20.0s    15534  canvas  16.5 GiB   +0 B      0 MiB   +0 B      ?
```

(`hmn` writes the stderr header line before the column header; the order above is how the two
streams arrived in the capture.)

The header names the criterion, as `dogfooding-watch-filter-by-identity.md` asked. The trainer's
binary is `/root/src/askesis/reference/canvas/target/release/canvas` (`nvidia-smi`'s
`process_name`); the NAME column shows `canvas`, and the substring match needed nothing longer.
The match is case-insensitive: `--filter CANVAS` selects the same process. With `--json`, the
first record is the `start` record, and its `selection` is complete:

```
{"kind":"start",...,"hmn_version":"0.2.12","argv":["hmn","watch","--filter","CANVAS","--min","1GiB",...],
 "selection":{"mode":"top","pids":[],"top":5,"filters":["CANVAS"],"min_bytes":1073741824}}
```

No match is a hard error that repeats the criterion, which is exactly right for a script:

```
$ hmn watch --filter nomatch --interval 5s --duration 11s     # rc=2
hmn: watch found no GPU processes on device 0 to auto-select (top 5 among names containing "nomatch" (case-insensitive)); re-run with an explicit PID once a workload is running
```

`ps`'s filters behave as documented too: `--pid 15534` returns the trainer, `--min 1GiB` keeps
it, `--min 20GiB` drops it, and an empty result echoes the filter
(`0 GPU processes found matching min=20 GiB.`). The echo means an empty table can never be
mistaken for an idle card. `hmn fits` is correct at both edges: `fits 10GiB` exits 0 with
`14.8 GiB free >= 10.0 GiB requested (4.83 GiB headroom)`, and `fits 20GiB` exits 1 with
`short by 5.17 GiB`.

## What was wrong — the Linux text summary states a negative it did not measure

The same `watch` run's text summary:

```
hmn watch: peak dedicated 0 MiB
           peak shared    0 MiB (baseline 0 MiB)
           episodes       0 — no spill observed
hmn watch: per-PID  PID    NAME    BASELINE COMMIT  PEAK COMMIT  BASELINE SHARED  PEAK SHARED
                    15534  canvas  16.5 GiB         16.5 GiB     0 MiB            0 MiB
```

and its JSON summary:

```
{"kind":"summary","measurable":false,"spilled":false,"observations":0,...,"peak_dedicated_bytes":0,"dedicated_limit_bytes":0,...}
```

Two lines of the text are misleading. The JSON is honest; the text drops its qualifier.

- **`peak dedicated 0 MiB`** sits two lines above a per-PID peak commit of 16.5 GiB. The 0 is
  the adapter-level spill sampler's value, and that sampler does not run on Linux
  (`observations: 0`). It is a placeholder, not a reading, but it reads as "the card's dedicated
  memory peaked at zero".
- **`no spill observed`** is the text form of `"spilled": false`. The JSON qualifies it with
  `"measurable": false`; the text does not. `hmn`'s other text output refuses this collapse.
  `hmn spill` on the same platform prints `spill not measurable on this platform` "instead of a
  misleading all-zeros report" (README, FAQ). `hmn ps`'s SPILL column prints `?`, never `no`,
  "so it can't be misread as 'measured, not spilling'" (`hmn --help`, Limitations). The `watch`
  summary prints the `no`.

For a reader of a `run.log` that this summary was pasted into, the line says the run was checked
for spill and had none. On this platform it was not checked.

## What made me work — `ps` selects by PID, and launchers hold the wrong PID

Our launcher (`remote.sh run`) starts `run_stages.sh` and reports its PID. The process holding
VRAM is a child:

```
canvas pid=15534 wrapper pid=15503
$ hmn ps --pid 15503
hmn: 0 GPU processes found matching pid=15503.
```

So `--pid` cannot answer the launcher's question ("is my job on the GPU?") without a `pgrep` in
front of it. `watch` gained `--filter` in v0.2.12 for exactly this reason. `ROADMAP.md` holds
the `ps` twin back until a fourth `ps` filter is requested, because adding one means refactoring
its filters into a shared `PsFilters` type. This report is that request. The tool is not wrong
here; it made me work.

## Campaign 2 — Windows: `ps` caught a real spill, and it was the trainer's own

To settle a question about the trainer's first-step loss, one training step was started locally at
the run's batch size (128) on the 5060 Ti, with Firefox open. `hmn ps` while it ran:

```
PID    NAME                            VRAM      SHARED   DEVICE                      SPILL
26476  canvas.exe                      14.2 GiB  1.9 GiB  NVIDIA GeForce RTX 5060 Ti  SPILL
22108  firefox.exe                     3.7 GiB   48 MiB   NVIDIA GeForce RTX 5060 Ti  SPILL
14996  dwm.exe                         545 MiB   2 MiB    NVIDIA GeForce RTX 5060 Ti  SPILL
4728   Zed.exe                         128 MiB   110 MiB  NVIDIA GeForce RTX 5060 Ti  SPILL
22756  csrss.exe                       75 MiB    0 MiB    NVIDIA GeForce RTX 5060 Ti  SPILL
...
12416  hmn.exe                         0 MiB     0 MiB    NVIDIA GeForce RTX 5060 Ti  SPILL
hmn: 25 GPU processes found (18.8 GiB committed total).
```

and the device line of a separate `hmn` run (the default subcommand) at about the same time:

```
GPU 0 [NVIDIA GeForce RTX 5060 Ti]: free 154 MiB / 16311 MiB (259 MiB reserved), driver 610.88
```

**The verdict was right, and the evidence for it is the SHARED column.** The trainer held 1.9 GiB
of shared residency, against a benign baseline of 48 MiB for the browser and 110 MiB for an
editor. The 18.8 GiB committed total is not evidence on its own: on WDDM, committed memory above
the card's size is normal (`hmn --help`, Limitations).

**The trainer does not fit on this card, with or without the desktop.** Its own footprint is
14.2 GiB dedicated + 1.9 GiB shared = 16.1 GiB here, and 16.5 GiB on the Linux 5090. The card has
16,311 MiB (15.9 GiB) in total, 259 MiB of it reserved by the driver. Spill starts lower still:
on this same card, Windows' video memory manager was measured to keep adapter-wide dedicated
residency at about 13.9–14.3 GiB, 88.6–91.3% of the capacity DXGI reports (FAQ, "Why is the
saturation threshold 85% and not 95% (or 100%)?"). Closing Firefox would not have made the
trainer fit. How much the browser added to the pressure cannot be read from this output: its
3.7 GiB is committed memory, and `hmn` does not show how much of a process's commit is resident.
`hmn fits 16.5GiB` would have said all this before launch: at most 16,052 MiB of the card can be
free, which is less than 16.5 GiB even with an empty desktop.

**The timings corroborate the spill; they do not prove it.** The step had not completed after
5 min on the GPU and was killed; `hmn` then read `free 15060 MiB / 16311 MiB`. The identical
step on the CPU (`--cpu`, same seed, same batch) completed in **249 s**, and a second CPU run
with one flag changed took 250 s, so the CPU timing is stable. The spilling GPU was therefore
slower than the CPU for this step, which is what spill predicts. Two limits apply. There is no
GPU run without spill to compare against: a smaller batch on this card, or the same step's
duration on the 5090. And free memory rising from 154 to 15,060 MiB after the kill only shows
that the trainer held that memory; it says nothing about spill either way. (Whether Firefox was
still open at that point was not recorded.)

**What made me work.** The SPILL column carries the device's verdict on every row, as the help
says (`spilling` is "broadcast per device"). That is honest, but the table does not say *who*.
`hmn.exe` at 0 MiB and `csrss.exe` at 75 MiB read `SPILL` exactly as `canvas.exe` does. Here the
paged process was obvious from SHARED (1.9 GiB against tens of MiB). But a reader skimming the
SPILL column, or a script reading `spilling` per row from `--json`, would conclude every process
on the card is spilling. The device-level fact belongs once on the summary line. The row-level
mark belongs on the process whose memory is being paged.

## Requests, most useful first

1. **Qualify the `watch` text summary when spill is not measurable.** When the summary's
   `measurable` is false, print what `hmn spill` prints on the same platform
   (`hmn watch: spill not measurable on this platform`) in place of the three
   `peak dedicated` / `peak shared` / `episodes` lines, and keep the per-PID block. Decide on
   `measurable`, not on `observations` being 0. Do not name a reason such as NVML: the same case
   arises on macOS and on Windows machines that cannot measure spill. The JSON already carries
   the truth; only the text needs to catch up. This is the change that matters most: it is a
   wrong statement in a log, not a missing feature.
2. **Say which process is being paged, on the row, and the device verdict once on the summary
   line.** Keep the per-device verdict, but state it once:
   `hmn: 25 GPU processes found (18.8 GiB committed total); device 0 SPILLING (154 MiB free, 2.1 GiB shared)`.
   Then mark the rows that hold a large share of the device's shared-resident bytes, and show the
   device verdict on the other rows in a quieter form. In `--json`, a per-row `shared_share`
   beside the broadcast `spilling` would let a script ask "is *my* process being paged?" without
   re-deriving it from `shared_used_bytes`. The mark must mean *paged*, not *cause*: the memory
   manager pages whatever it chooses, and the process being paged need not be the one that
   pushed the card over. `dogfooding-spill-triage-watch-mode.md`'s third verdict is that case:
   new desktop tenants grew their commit and caused the pressure, while the paging landed mostly
   on idle tenants and brushed the trainer (224 MiB shared, a 15% slowdown). A `CAUSE` column
   keyed on shared residency would have named the idle tenants there.
3. **`hmn ps --filter <PATTERN>`**, with exactly `watch --filter`'s semantics: case-insensitive
   substring, repeatable (OR), and the criterion echoed on the summary line. This is the `ps`
   filter this workload would use daily, in place of `pgrep -x canvas` + `--pid`.
4. **An opt-in exit status for "nothing matched" on `ps`** (`--exit-status`, as `pgrep`: 0 if a
   row matched, 1 if none). Then `hmn ps --filter canvas --exit-status || echo "not on the GPU"`
   is a one-line gate, as `hmn fits` already is for headroom. Today `ps` exits 0 on no match,
   while `watch` exits 2 on no match. Both are defensible alone; together they surprise.
5. **`ps --device` out of range should be an error, as it is in `fits`.** `hmn fits 1GiB --device 3`
   exits 2 with `device index 3 out of range (have 1 devices)`; `hmn ps --device 3` exits 0 with
   `0 GPU processes found matching device=3.` A mistyped index then reads as an idle card.
6. **Repeatable `--pid`** (`--pid A --pid B`, OR), for a parent and its child or two chained runs.
   Today it is rejected cleanly (`error: the argument '--pid <PID>' cannot be used multiple times`,
   exit 2), but it is rejected.

## Smaller observations

1. **`watch` with an explicit PID that does not exist runs as if it existed.**
   `hmn watch 999999 --interval 1s --duration 2s` exits 0, prints rows `999999  ?  0 MiB`, and a
   per-PID summary of zeros. `hmn watch` documents that it cannot tell "exited" from "holds no
   VRAM" during a watch, and a process that exists but holds no VRAM yet (a trainer between
   stages) must be watchable. The request is narrower: at attach, a PID that does not exist at
   all is a checkable fact. A one-shot stderr line (`PID 999999 does not exist`) would separate a
   mistyped PID from an idle process.
2. **`watch`'s text columns do not align under their header.** Header
   `PID     NAME          COMMITTED`, row `15534  canvas  16.5 GiB`: the header is printed once
   with fixed widths, and each sample's rows are sized to that sample's contents, so rows line up
   only when their cells happen to fill the header's widths. Nothing about this is specific to
   Linux. Cosmetic.
3. **The top-level `hmn --help` is several screens**, most of it the per-platform Limitations,
   which pushes the subcommand list below the fold. A short `-h` pointing to the long form, or a
   `hmn help limitations` topic, would keep the front page scannable. Cosmetic.
4. **Benign baseline for a Linux trainer:** SHARED 0 MiB throughout (always 0 on Linux by design),
   COMMITTED flat at 16.5 GiB (`ΔCOMMIT +0 B` over 25 s mid-epoch), 14.8 GiB free of 32 GiB.

## Acceptance fixtures (already run, free to regress against)

| Command (Linux, NVML, one 16.5 GiB process `canvas`, unless stated) | Measured | Expected |
|---|---|---|
| `hmn ps` | 1 row, 16.5 GiB, SPILL `?`, rc 0 | same |
| `hmn ps --pid <canvas>` / `--pid <wrapper>` | 1 row / 0 rows + `matching pid=…`, rc 0 / 0 | same (rc 1 under request 4) |
| `hmn ps --min 1GiB` / `--min 20GiB` | 1 row / 0 rows + `matching min=20 GiB`, rc 0 | same |
| `hmn ps --device 3` (1 device) | 0 rows, rc 0 | rc 2, out-of-range error (request 5) |
| `hmn ps --pid 1 --pid 2` / `--pid 1,2` | rc 2, clap error | rc 0, OR (request 6) / rc 2 |
| `hmn fits 10GiB` / `fits 20GiB` | rc 0 / rc 1, headroom and shortfall lines | same |
| `hmn fits 1GiB --device 3` | rc 2, `out of range (have 1 devices)` | same |
| `hmn watch --filter canvas` | selects PID 15534, criterion on header | same |
| `hmn watch --filter CANVAS --min 1GiB --json` | `start` record with `filters`, `min_bytes`; samples; summary `measurable:false` | same |
| `hmn watch --filter nomatch` / `--min 20GiB` | rc 2, criterion repeated in the error | same |
| `hmn watch --filter canvas` text summary | `no spill observed`, `peak dedicated 0 MiB` | `spill not measurable on this platform`, no peak lines (request 1) |
| `hmn watch 999999` (no such PID) | rc 0, rows of `?  0 MiB` | a "does not exist" notice at attach (observation 1) |
| Windows: `hmn ps` during a batch-128 step on a 16 GiB card | `canvas.exe` 14.2 GiB + 1.9 GiB shared, SPILL on all 25 rows; `hmn`: 154 MiB free | SPILL verdict ✓; `canvas.exe` marked as the paged row, verdict stated once on the summary (request 2) |
| Windows: `hmn` after killing it | `free 15060 MiB / 16311 MiB` | same |

## Confidence

High for every measured value above. The VRAM figure has an independent second signal,
`nvidia-smi` at the same moment: `15534, …/canvas, 16912 MiB` against `hmn`'s 16.5 GiB, which
agree within `hmn`'s display rounding (16.5 GiB covers 16,845–16,947 MiB). `nvidia-smi`'s
`memory.used 16922 MiB` against `hmn`'s `free 15188 MiB / 32607 MiB` leaves a 497 MiB gap. That
is consistent with the driver-reserved memory, which NVML counts in neither `used` nor `free`
(`dogfooding-candle-mi-nvml-reserved.md`); the `reserved` figure on `hmn`'s line was not
captured to confirm it. All Linux output was pasted from the box while the run
was live.

The "not measurable" reading of the Linux summary is corroborated by the tool itself
(`"measurable": false`, `"observations": 0`) and by its documentation (SHARED "always 0 on
Linux"; `hmn spill`'s own "not measurable" line). `peak dedicated 0 MiB` is the one value in
this report that is a placeholder rather than a measurement, which is the finding.

The Windows spill verdict rests on the trainer's 1.9 GiB of shared residency, and on its
16.1 GiB footprint exceeding the card's 15.9 GiB (confirmed independently by the 16.5 GiB it
holds on Linux). The timings (>300 s on the spilling GPU, 249 s and 250 s on the CPU) are
consistent with spill, but without a non-spilling GPU run they corroborate it rather than prove
it. The Windows `ps` table and the `154 MiB free` line come from two commands run moments apart;
no conclusion above depends on combining them. The wording judgements (requests 1 and 2,
observation 1) are mine.

## References

- [`dogfooding-watch-filter-by-identity.md`](dogfooding-watch-filter-by-identity.md): the request
  that became `watch --filter`, validated here on Linux.
- [`dogfooding-orphan-attribution-and-ps-spill-flag.md`](dogfooding-orphan-attribution-and-ps-spill-flag.md):
  the `?`-not-`no` SPILL rule that request 1 asks the `watch` summary to follow, and the `ps`
  SPILL flag whose attribution request 2 refines.
- [`dogfooding-spill-triage-watch-mode.md`](dogfooding-spill-triage-watch-mode.md), Verdict 3: a
  spill caused by new desktop tenants but paged mostly onto idle ones — why request 2's mark
  must mean *paged*, not *cause*.
- [`dogfooding-candle-mi-nvml-reserved.md`](dogfooding-candle-mi-nvml-reserved.md): the reserved
  memory that accounts for the gap between `nvidia-smi`'s `memory.used` and `hmn`'s free figure.
- `docs/FAQ.md`, "Why is everything spill-related 0 / `false` on Linux and macOS?" (`hmn spill`'s
  "not measurable" line, the precedent for request 1) and "Why is the saturation threshold 85%
  and not 95% (or 100%)?" (the dedicated-residency ceiling measured on the 5060 Ti, 88.6–91.3%
  of the capacity DXGI reports).
- `docs/roadmap-v0.2.12.md`, Part 2, "`hmn watch` only", and `ROADMAP.md`'s `PsFilters` entry:
  the deferral of a `ps` filter that request 3 asks to schedule.
- askesis: `reference/canvas/remote.sh` (`status` → `hmn ps`), `run_stages.sh` (per-stage `hmn`
  census); the run is `acspv_pl_s1_d1`, rentals row of 2026-09-28 in
  `reference/canvas/docs/rentals.md` (branch `acsp14`).
