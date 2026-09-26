# Dogfooding report (from candle-mi): `--follow-new` fixed *when* `watch` selects, but selection is still by VRAM rank, so 74% of a committed research artifact is my desktop

**Date:** 2026-09-21
**Reporter:** candle-mi Figure-13 patching campaign (`examples/figure13_newline_patch`, gemma-2-2b and Llama-3.2-1B, RTX 5060 Ti 16 GiB, Windows 11 / WDDM; `probe`/`validate` captured 2026-09-14 on `hmn` ≤ `0.2.10`, `grid`/`midline` captured 2026-09-15 on `0.2.11` — the two later files carry the `wall_clock` field v0.2.11 added, the two earlier ones do not; `hmn` on PATH today is `0.2.11`)
**Severity:** Field validation of v0.2.7 `--follow-new` (it worked exactly as specified) + one request for **v0.2.12**: select by process identity, not only by VRAM rank
**Affected area:** `hmn watch` auto-selection criterion. `--follow-new` re-runs the top-N choice every interval; the *criterion* is still committed-VRAM descending, which cannot express "the process I am measuring"
**Status:** ✅ **Resolved in v0.2.12** (2026-09-26). All three requests shipped as asked — `watch --filter` (case-insensitive substring, repeatable, composing with `--follow-new`), `watch --min` with `ps --min`'s footprint, the active criterion on the header line — and smaller observation 1 as a `{"kind":"start",...}` record, always the first line of `--json` (the release's one deliberate wire-format addition). Observation 2's churn is what `--filter` removes. Reviewing the work also found Linux names cut to 15 bytes, which would have kept this report's own `--filter figure13_newline_patch` from matching there; fixed in the same release.

---

## TL;DR

Four `hmn watch --follow-new --top 3` captures (`--interval 30s`, except `probe` at 15s) ran
alongside a Figure-13 activation patching campaign and are committed in candle-mi's public
repository as experimental record. The v0.2.7 machinery worked: the followed set tracked every
short-lived `figure13_newline_patch.exe` process across its birth and death, 34 in `grid` and 7 in
`midline` (the two captures that reached a `summary`), which is exactly the failure the
`--follow-new` report asked it to fix. The adapter verdicts were correct too: no capture
spilled. `grid` and `midline`, the two that closed with a `summary`, peaked at 93.3% and 88.9%
of the dedicated limit with `episodes: []`; `probe` and `validate` were hard-killed before their
summary, so for those the evidence is that **none of their 426 sample rows carries
`"spilling":true`** — as none of all 1,866 does.

But **487 of 1,866 sample rows are the workload. The other 1,379 (73.9%) are `dwm.exe`,
`firefox.exe`, `SamsungMagician.exe`, `claude.exe`, `Code.exe`, `Discord.exe` and one lone
sighting of `Twake Desktop.exe`.** That is with `--top 3` applied. `--top 1` is legal and would
have done better than this report first claimed — see the counterfactual below, 78.3% workload
overall and 96.7% in `grid` — but it buys that by giving up co-tenant context entirely, and it
still records the desktop whenever the workload is between processes. Ranking by VRAM cannot say
"follow this program" at any `N`: a compositor holding 1.16 GiB (peaking at 1.70 GiB) outranks
nothing and never leaves.

The cost is not only noise. These files are committed to a public repository, so the capture
published the names of a messaging client, a browser and an SSD utility that had nothing to do
with the experiment.

## What worked: `--follow-new` did exactly what the previous report asked

The v0.2.7 request was that a frozen PID set never saw the processes that mattered, because
they were born after the first sample. That is fixed, and the `.err` stream shows it working.
`hmn_watch_grid.err`'s header line and the first 7 of its 42 handovers, verbatim:

```
hmn watch: device 0 [NVIDIA GeForce RTX 5060 Ti], interval 30.0s, following top 3 by committed (re-selected every interval), 3 initially
hmn watch: +30.6s followed set changed: entered pid=15032 (figure13_newline_patch.exe); left pid=11972 (SamsungMagician.exe)
hmn watch: +331.1s followed set changed: entered pid=13904 (figure13_newline_patch.exe); left pid=15032 (figure13_newline_patch.exe)
hmn watch: +631.7s followed set changed: entered pid=17680 (figure13_newline_patch.exe); left pid=13904 (figure13_newline_patch.exe)
hmn watch: +902.1s followed set changed: entered pid=18848 (figure13_newline_patch.exe); left pid=17680 (figure13_newline_patch.exe)
hmn watch: +1202.6s followed set changed: entered pid=15380 (figure13_newline_patch.exe); left pid=18848 (figure13_newline_patch.exe)
hmn watch: +1473.1s followed set changed: entered pid=16428 (figure13_newline_patch.exe); left pid=15380 (figure13_newline_patch.exe)
hmn watch: +1803.7s followed set changed: entered pid=18032 (figure13_newline_patch.exe); left pid=16428 (figure13_newline_patch.exe)
```

Each line names the entering and the leaving PID. Across the whole `grid` capture that is 42
handovers, the last at +176.8 min of a 179.8-minute capture (360 observations × 30s), 34 of
which admitted a `figure13_newline_patch.exe`. Every one of the campaign's processes was picked
up on the interval after it started, and finalized rather than rendering zeros when it exited.
The `entered ... left ...` line is the right amount of information and should not change.

The per-PID finalization is also correct. From the shorter `hmn_watch_midline.jsonl`, whose
`summary` carries all eleven PIDs that run ever followed, 7 of them workload, with per-PID
baseline and peak, so a process that lived for four intervals is still accounted for:

```json
{"kind":"summary","measurable":true,"spilled":false,"observations":120,
 "baseline_shared_bytes":85893120,"peak_shared_bytes":168259584,
 "peak_dedicated_bytes":14966763520,"dedicated_limit_bytes":16831741952,
 "total_spill_duration_ms":0,"episodes":[],"per_pid":[ ... 11 entries ... ]}
```

**Quantifying the benign baseline, for the next reader — and mind which quantity each figure
is.** The per-process number is dedicated **commit** (PDH `VidMm` reservation, which may
legitimately exceed physical VRAM); the adapter numbers are **resident**. They are not
commensurable, so read them as two anchors, not one scale. One workload process peaks at
13.23 GB *committed*, **78.6%** of the 16.83 GB (15.67 GiB) dedicated limit, with its own shared
at 81.8 MB. Adapter-wide the *resident* peak reached 14.97 GB (88.9%) in `midline` and 15.70 GB
(93.3%) in `grid`, with peak shared at 168 to 172 MB, almost exactly **1.0%** of the dedicated
limit in both. Neither spilled. So on this card, a single process committing ~79% of dedicated,
an adapter resident peak up to ~93%, and shared holding near 1%, is the *healthy* signature for
a 2B-parameter patching run, and it is the anchor against which a future climb should be read.

## What was missing: rank is a proxy for identity, and a bad one

`--top 3` was chosen deliberately to keep the record tight. It was not enough, because at any
instant the top three by committed VRAM are the one live `figure13_newline_patch.exe`, the
compositor, and whatever else the desktop is doing. Counted across all four committed captures:

| capture | workload rows | total sample rows | workload share |
|---|---|---|---|
| `hmn_watch_grid.jsonl` | 348 | 1080 | 32.2% |
| `hmn_watch_midline.jsonl` | 73 | 360 | 20.3% |
| `hmn_watch_probe.jsonl` | 20 | 159 | 12.6% |
| `hmn_watch_validate.jsonl` | 46 | 267 | 17.2% |
| **total** | **487** | **1866** | **26.1%** |

**`--top 1` was available, and this report originally claimed it was not.** `--top 3` is not the
narrowest the interface allows; `--top N` takes any `N`. That matters here more than it should,
because the workload was **rank 1 at every one of the 487 timestamps it appeared** — the campaign
never ran two patch processes at once, so `--top 1` would never have missed a live one:

| capture | workload share at `--top 3` (observed) | at `--top 1` (counterfactual) |
|---|---|---|
| `hmn_watch_grid.jsonl` | 32.2% | **96.7%** (348/360) |
| `hmn_watch_midline.jsonl` | 20.3% | 60.8% (73/120) |
| `hmn_watch_probe.jsonl` | 12.6% | 37.7% (20/53) |
| `hmn_watch_validate.jsonl` | 17.2% | 51.7% (46/89) |
| **total** | **26.1%** | **78.3%** (487/622) |

So the 73.9% figure in this report's title is a consequence of choosing `--top 3`, not of the
interface having no narrower setting, and the honest baseline for the request below is 78.3%, not
26.1%. What `--top 1` cannot do is reach 100%: the residual 135 rows are the desktop recorded
while the workload was between processes, and `--top 1` also discards exactly the co-tenant
context that made the *previous* two reports diagnosable. The request stands — rank cannot
express identity at any `N` — but it is a request for the last 22 points, plus stability, plus
the ability to keep `--top 3` and still record only what was asked for.

Distinct names recorded, and how many sample rows each occupies:

```
  622  dwm.exe
  487  figure13_newline_patch.exe     <- the only one the experiment is about
  388  firefox.exe
  184  SamsungMagician.exe
  158  claude.exe
   13  Code.exe
   13  Discord.exe
    1  Twake Desktop.exe
```

This is the "made me work" kind of finding, not the "tool is wrong" kind. Ranking processes by
committed VRAM is correct, and it is the right default when you do not know what you are
looking for. The gap is that when you *do* know, there is no way to say so. The only
identity-based selection is an explicit PID list, and this campaign's PIDs could not be known
in advance because the processes were launched sequentially by a driver script, 34 of them in
the `grid` capture alone.
`--follow-new` exists precisely for processes that do not exist yet; it simply cannot be told
*which* ones.

**`--min <SIZE>` would *not* have worked cleanly here, and the way it fails is the real
argument.** This report first claimed `--min 2GiB` would have isolated the workload, reasoning
from 13.2 GB against `dwm.exe`'s 1.16 GiB. Checked against the files, that is wrong: `firefox.exe`
reaches **2.99 GiB** in `probe` and 2.97 GiB in `validate`, 49 rows at or above 2 GiB. The
counterfactuals:

| threshold | workload rows kept | non-workload rows admitted | share |
|---|---|---|---|
| `--min 2GiB` | 487 of 487 | **49** (`firefox.exe`) | 90.9% |
| `--min 3GiB` | **482 of 487** | 0 | 100% |

`--min 3GiB` is clean only by silently dropping five genuine workload rows — the early-life
samples where the process had committed as little as 2.40 GiB on its way up. So no single
threshold on this data is both complete and clean, and the two failure modes are on opposite
sides of it. That is worse than the "works only when the workload is enormous" caveat this
report first gave, and it is the same point made harder: a size filter is a proxy for identity,
and it misses in both directions even in the case that *should* be easy. On an AlgZoo tiny model
or any run under a gigabyte it does not even have that.

## The privacy consequence, which is specific to how these files are used

`hmn watch --json` output is *research record*. candle-mi commits it: four of these files live
in [`docs/experiments/figure13-patching/`](https://github.com/mi-for-the-rust-of-us/candle-mi/tree/main/docs/experiments/figure13-patching)
in a public repository, alongside the `spec.md` and the per-cell JSON they corroborate. That is
the intended use, and the reason the `--json` flag is valuable.

The result is that a memory-measurement run published `Discord.exe`, `Twake Desktop.exe`,
`firefox.exe` and `SamsungMagician.exe` to a public repository. Nothing here is a secret and
this is not a security issue. But a tool whose output is designed to be committed should make
it possible to record only the process under study, and today it does not. `Twake Desktop.exe`
appears in exactly one row out of 1,866, which is the shape of the problem: a single incidental
sighting, permanent.

Worth stating as the counter-example, because it shows the project already has the right
instinct elsewhere: **`hmn spill` records no process names at all.** Its `SpillReport` is
adapter-level, with no `per_pid` array, which is why candle-mi's `resurrect.ps1` integration
(`hmn spill --json` around the oracle entries marked `Spill = $true`) produces clean,
committable output with no desktop in it. The two commands currently trade privacy against
detail, and neither offers "per-PID detail, for my process only".

## Requests, ordered by how much they would have helped

1. **`--filter <SUBSTRING>` on `watch`, composing with `--follow-new`.** This is the one that
   matters; the others are conveniences.

       hmn watch --follow-new --filter figure13_newline_patch --interval 30s --json

   Semantics: restrict auto-selection to processes whose resolved name matches, then apply
   `--top N` within that set. It narrows the existing criterion rather than adding a selection
   mode, so `--follow-new`'s entered/left bookkeeping is unchanged. Combined with an explicit
   PID list it should be a hard error, exit `2`, exactly as `--follow-new` with explicit PIDs
   already is: there is no top-N to filter against a fixed list.

   Repeatable (`--filter a --filter b`) would cover a campaign spanning two binaries. One
   occurrence is enough for this workload.

   **One failure mode a filter introduces, which rank-based selection does not have.**
   `GpuProcessEntry::name` is `Option<String>`, and on Windows can arrive as `[protected]`,
   `[exited]` or `?`. Under ranking, a name-resolution miss costs a *label*: the row is still
   selected, still baselined, still measured. Under `--filter` it costs the *process*: an
   unresolved row can never match, so it is never selected, never baselined, and nothing in the
   artifact says a row was dropped — the silent-empty-capture shape. `ROADMAP.md` already tracks
   an open name-resolution race on exactly this axis, and the report that surfaced it saw a
   `firefox.exe` row peaking at an implausible 15.7 GB. It did not bite this campaign (zero
   unresolved rows across all 1,866), so this is a design note rather than an objection, but the
   filter should either count unresolved-and-therefore-unmatched rows on the header/stderr line,
   or match against the sticky last-resolved name rather than the current sample's.

2. **`--min <SIZE>` on `watch`,** matching `ps --min`. Useful in its own right and trivially
   composable with request 1, but see the counterfactuals above: on this campaign's own data no
   threshold is both complete and clean — 2 GiB admits `firefox.exe`, 3 GiB drops five real
   workload rows — and on a workload under a gigabyte it separates nothing at all. Worth having
   for headroom questions, not worth having *instead* of request 1.

3. **Say in the `.err` header which criterion is active.** The header today reads `following
   top 3 by committed (re-selected every interval)`. If a filter is applied it should say so,
   because that line is what a reader of a committed artifact uses to reconstruct what was
   captured, and a filtered capture that does not announce its filter is an artifact nobody can
   interpret later.

## Smaller observations

1. **A hard-killed `watch` writes no summary, and the artifact does not say it was truncated.**
   Two of the four committed captures (`hmn_watch_probe.jsonl`, `hmn_watch_validate.jsonl`) end
   on a `sample` record with no `summary`. A third, `hmn_watch_midline.jsonl`, was committed
   mid-capture and only completed in candle-mi on 2026-09-21. Nothing in the file marks an
   incomplete run as incomplete, so a reader cannot distinguish "the run ended here" from "the
   file was cut here". A `{"kind":"start", ...}` record written at attach, carrying the
   invocation and the criterion, would make truncation detectable from the file alone. Cheap,
   and it also serves request 3 for consumers that keep the `.json` and discard the `.err`.
   **Note for whoever schedules this:** it is the one ask here that is not additive *in effect*.
   The three numbered requests are new off-by-default flags and a header line that only changes
   when one of them is passed; a `start` record changes the default `--json` stream for every
   existing consumer, including candle-mi's own parsing. Consumers that filter on `kind` are
   fine; anything that assumes the first line is a `sample` is not. Worth either gating it behind
   a flag or shipping it as the one deliberate wire-format change of the release.

2. **`--top 3` with `--follow-new` produces rapid churn when the workload restarts.** In
   `grid`, 42 handovers across a 179.8-minute capture; 29 of them are workload→workload, one
   patch process evicting the previous one as it starts, and 34 have a workload process leaving.
   Only 8 admit something that is not the workload. The bookkeeping is correct, and a filter
   would have removed those 8 and held the followed set at one row, which is easier to read.
   **Not cheaper, though** — this report first claimed a filter would save PDH queries and that
   is wrong: `gpu_processes(device)` enumerates every process every interval regardless of
   `--top`, spill tracking is one adapter-level observation per interval, and the per-PID step is
   a linear scan over rows already fetched. Narrowing the followed set costs and saves nothing at
   the query layer; the argument for a filter is legibility and privacy, not cost.

## Acceptance fixtures (already run, free to regress against)

| case | setup | expected | observed 2026-09-15 |
|---|---|---|---|
| Sequential short-lived processes are all picked up (`grid`) | `--follow-new --top 3`, 34 launches inside a 179.8-min capture | each enters within one interval of starting | 34 of 34, first at +30.6s; each entry within ±15s of the launch time `run.log` records |
| Departed PIDs are finalized, not zero-filled (`midline`) | `--follow-new --top 3`, 120 observations | every followed PID in `summary.per_pid` with peak and baseline | 11 PIDs, 7 of them workload, all with both |
| Adapter verdict near saturation | peak 15.70 GB of 16.83 GB (`grid`) | `spilled: false`, `episodes: []` | as expected, `total_spill_duration_ms: 0` in `grid` and `midline` (the only two that closed with a summary); `"spilling":true` in 0 of 1,866 sample rows |
| Healthy signature, 2B patching run | gemma-2-2b, RTX 5060 Ti 16 GiB | one process ~79% dedicated *commit*, adapter shared ~1% | 13.23 GB peak committed (78.6%), shared peak 168-172 MB (1.0%) |
| Noise share at `--top 3` | `--top 3 --follow-new`, desktop active | (no expectation; this is the finding) | 26.1% workload, 73.9% not |
| Noise share at the actual narrowest setting | `--top 1 --follow-new`, same rows re-ranked | (counterfactual, computed from the committed files) | 78.3% workload overall, 96.7% in `grid` |
| No PID-reuse contamination of the name counts | `grid` + `validate` `.err` | no `name changed` breadcrumb, no unresolved-growth hint | 0 and 0; no non-workload row above 2.99 GiB, so no row is the workload under another name |

The last two noise rows are the regression case for request 1: with `--filter
figure13_newline_patch` the workload share should be 100% at any `--top N`, and the fixture is the
same campaign re-run. The `--top 1` row is the honest baseline it has to beat.

## Confidence

**High on the counts, which are mechanical.** They come from parsing the four committed
`.jsonl` files directly, not from reading the human-readable stream: every `{"kind":"sample"}`
record carries a `name`, so the shares are exact rather than estimated.

**Independently corroborated on three axes.** The `.err` stream is a second, separately written
record of the same run, and for `grid` the 34 `followed set changed` lines that admit a
`figure13_newline_patch.exe` name exactly the same 34 PIDs, in the same order, as that
capture's own `summary.per_pid` array. (Checked within one capture deliberately: an earlier
draft of this report cross-referenced `grid`'s `.err` against `midline`'s summary, which are
different runs with disjoint PIDs, and the apparent agreement was an artifact of quoting only
the first 8 lines.) Second, the campaign's own `run.log` — written by the driver script, not by
`hmn` — records exactly **34 cells whose start time falls inside the `grid` capture window**
(16:33:24 → 19:29:50), and those 34 launches align to the 34 `.err` entries within **±15s**, half
an interval. That takes "34 of 34" out of the instrument's own bookkeeping, where this report
originally left it. Third, the no-spill verdict is corroborated by the experiment's outcome: the
campaign completed without the roughly 15x slowdown that candle-mi's oracle suite records for a
genuine spill (`longrope`, Phi-3.5-mini at F32, 8,479 MiB growth over 13m18s), so "did not spill"
is confirmed by wall-clock as well as by the instrument.

**The name counts are not contaminated by PID reuse, which was worth checking.** `hmn watch`
resets a row's baseline when a watched PID's resolved name changes, and the predecessor report saw
a misattributed `firefox.exe` row peaking at an implausible 15.7 GB. Across these captures there
are **zero `name changed` breadcrumbs and zero unresolved-growth hints**, and no non-workload row
exceeds 2.99 GiB (`firefox.exe`) against the workload's 12.33 GiB — so no row in the 1,379 is the
workload wearing another process's name, and the 26.1% / 78.3% figures are not an artifact of
misattribution.

**Five things this report got wrong, corrected 2026-09-26 by a hypomnesis-side validation pass
that re-derived every number from the committed files and checked every behavioural claim against
the v0.2.11 source.** Recorded rather than silently edited, because the folder is a record. (1)
`--top 3` is *not* the narrowest legal selection — `--top 1` exists, and would have given 78.3%
workload rather than 26.1%, which cuts the headline severity by a factor of three. (2) `--min
2GiB` would *not* have cleanly isolated the workload: `firefox.exe` reaches 2.99 GiB, and no
threshold on this data is both complete and clean. (3) `total_spill_duration_ms: 0` was claimed
"in all four" captures; only two carry a `summary` at all, and the right citation is the 0-of-1,866
`"spilling":true` rows. (4) A filter would *not* be "cheaper in PDH queries" — nothing in the
sampling path scales with the followed-set size. (5) `probe` ran at `--interval 15s`, not 30s, and
the four captures were not all produced by the same `hmn` build. Every other count, quotation and
behavioural claim in this report was confirmed exact.

**One thing I got wrong at the instrument, recorded because the next reader will repeat it.** I
first attached `hmn watch <pid>` to an already-running, already-saturated process and read its
`episodes 0, no spill observed` as contradicting a known spill. It was not: `watch` takes its
baseline from the first sample, so growth before the attach is baseline, not an episode. The tool
was right, and the closing summary says `baseline 8.3 GiB` plainly. This is an argument for
observation 1 rather than a complaint: a `start` record naming the attach time would have made
the late attach self-evident in the artifact.

## References

- [`dogfooding-watch-follow-new.md`](dogfooding-watch-follow-new.md), the direct predecessor.
  It fixed *when* selection happens; this report is about *what* selection is based on. Same
  class of problem, the instrument tracking the desktop rather than the workload, on the other
  axis.
- [`dogfooding-spill-triage-watch-mode.md`](dogfooding-spill-triage-watch-mode.md), for
  tenant-driven versus workload-driven as the discriminating diagnosis, which is why per-PID
  attribution exists at all and therefore why a filter is worth having.
- candle-mi's committed artifacts, `docs/experiments/figure13-patching/hmn_watch_*.jsonl`, and
  the campaign's own `spec.md`, which cites `hmn watch --follow-new` for its no-spill claim.
- candle-mi's `docs/experiments/figure13-patching/run.log`, the driver script's own launch record.
  It is the independent second source for "34 of 34" and is the file to re-parse if this report's
  counts are ever doubted.
- candle-mi's `scripts/resurrect.ps1`, for the contrasting integration: `hmn spill --json`
  wrapped around oracle entries marked `Spill = $true`, which records no process names and so
  needs no filter.
