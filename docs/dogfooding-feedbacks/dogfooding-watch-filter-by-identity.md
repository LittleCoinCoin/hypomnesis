# Dogfooding report (from candle-mi): `--follow-new` fixed *when* `watch` selects, but selection is still by VRAM rank, so 74% of a committed research artifact is my desktop

**Date:** 2026-09-21
**Reporter:** candle-mi Figure-13 patching campaign (`examples/figure13_newline_patch`, gemma-2-2b and Llama-3.2-1B, RTX 5060 Ti 16 GiB, Windows 11 / WDDM; captures taken 2026-09-15, `hmn` on PATH today is `0.2.11`)
**Severity:** Field validation of v0.2.7 `--follow-new` (it worked exactly as specified) + one request for **v0.2.12**: select by process identity, not only by VRAM rank
**Affected area:** `hmn watch` auto-selection criterion. `--follow-new` re-runs the top-N choice every interval; the *criterion* is still committed-VRAM descending, which cannot express "the process I am measuring"
**Status:** Proposed, v0.2.12 candidate

---

## TL;DR

Four `hmn watch --follow-new --top 3 --interval 30s` captures ran alongside a Figure-13
activation patching campaign and are committed in candle-mi's public repository as
experimental record. The v0.2.7 machinery worked: the followed set tracked every short-lived
`figure13_newline_patch.exe` process across its birth and death, 34 of them in `grid` and 7 in
`midline` (the two captures that reached a `summary`), which is exactly the failure the
`--follow-new` report asked it to fix. The adapter verdicts were correct too: no capture spilled, peaking at 93.3% of the
dedicated limit in `grid` and 88.9% in `midline`, with zero episodes.

But **487 of 1,866 sample rows are the workload. The other 1,379 (73.9%) are `dwm.exe`,
`firefox.exe`, `SamsungMagician.exe`, `claude.exe`, `Code.exe`, `Discord.exe` and one lone
sighting of `Twake Desktop.exe`.** That is with `--top 3` already applied, which is the
narrowest the interface allows. Ranking by VRAM cannot say "follow this program", so a
compositor holding 1.16 GiB outranks nothing and never leaves.

The cost is not only noise. These files are committed to a public repository, so the capture
published the names of a messaging client, a browser and an SSD utility that had nothing to do
with the experiment.

## What worked: `--follow-new` did exactly what the previous report asked

The v0.2.7 request was that a frozen PID set never saw the processes that mattered, because
they were born after the first sample. That is fixed, and the `.err` stream shows it working.
The first 8 of `hmn_watch_grid.err`'s 42 handover lines, verbatim:

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
handovers over 176.8 minutes, 34 of which admitted a `figure13_newline_patch.exe`. Every one of
the campaign's processes was picked up on the interval after it started, and finalized rather
than rendering zeros when it exited. The `entered ... left ...` line is the right amount of
information and should not change.

The per-PID finalization is also correct. From the shorter `hmn_watch_midline.jsonl`, whose
`summary` carries all eleven PIDs that run ever followed, 7 of them workload, with per-PID
baseline and peak, so a process that lived for four intervals is still accounted for:

```json
{"kind":"summary","measurable":true,"spilled":false,"observations":120,
 "baseline_shared_bytes":85893120,"peak_shared_bytes":168259584,
 "peak_dedicated_bytes":14966763520,"dedicated_limit_bytes":16831741952,
 "total_spill_duration_ms":0,"episodes":[]}
```

**Quantifying the benign baseline, for the next reader:** one workload process peaks at
13.23 GB committed against a 16.83 GB (15.67 GiB) dedicated limit, **78.6%**, with its own
shared at 81.8 MB. Adapter-wide the peak reached 14.97 GB (88.9%) in `midline` and 15.70 GB
(93.3%) in `grid`, with peak shared at 168 to 172 MB, almost exactly **1.0%** of the dedicated
limit in both. Neither spilled. So on this card, a single process at ~79% of dedicated, an
adapter peak up to ~93%, and shared holding near 1%, is the *healthy* signature for a
2B-parameter patching run, and it is the anchor against which a future climb should be read.

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

**`--min <SIZE>` would have worked here, and that is the trap.** At 13.2 GB against
`dwm.exe`'s 1.16 GiB, `--min 2GiB` would have cleanly isolated the workload. But it only works
because this workload is enormous. The same campaign on an AlgZoo tiny model, or any run under
a gigabyte, is outranked by the compositor and no threshold separates them. A size filter is a
proxy that happens to work when the workload is the biggest thing on the card, which is exactly
the case where you least need the help.

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

2. **`--min <SIZE>` on `watch`,** matching `ps --min`. Useful in its own right and trivially
   composable with request 1, but see the caveat above: it is a proxy that fails exactly when
   the workload is small. Worth having, not worth having *instead*.

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

2. **`--top 3` with `--follow-new` produces rapid churn when the workload restarts.** In
   `grid`, 42 handovers over 176.8 minutes, most of them evicting the previous workload process
   as the next one starts. The bookkeeping is correct, but a filter would have removed the
   competition entirely and kept the followed set stable at one row, which is easier to read
   and cheaper in PDH queries.

## Acceptance fixtures (already run, free to regress against)

| case | setup | expected | observed 2026-09-15 |
|---|---|---|---|
| Sequential short-lived processes are all picked up (`grid`) | `--follow-new --top 3`, 34 launches over 176.8 min | each enters within one interval of starting | 34 of 34, first at +30.6s |
| Departed PIDs are finalized, not zero-filled (`midline`) | `--follow-new --top 3`, 120 observations | every followed PID in `summary.per_pid` with peak and baseline | 11 PIDs, 7 of them workload, all with both |
| Adapter verdict near saturation | peak 15.70 GB of 16.83 GB (`grid`) | `spilled: false`, `episodes: []` | as expected, `total_spill_duration_ms: 0` in all four |
| Healthy signature, 2B patching run | gemma-2-2b, RTX 5060 Ti 16 GiB | one process ~79% dedicated, shared ~1% | 13.23 GB peak committed (78.6%), shared peak 168-172 MB (1.0%) |
| Noise share at the narrowest legal selection | `--top 3 --follow-new`, desktop active | (no expectation; this is the finding) | 26.1% workload, 73.9% not |

The last row is the regression case for request 1: with `--filter figure13_newline_patch` the
workload share should be 100%, and the fixture is the same campaign re-run.

## Confidence

**High on the counts, which are mechanical.** They come from parsing the four committed
`.jsonl` files directly, not from reading the human-readable stream: every `{"kind":"sample"}`
record carries a `name`, so the shares are exact rather than estimated.

**Independently corroborated on two axes.** The `.err` stream is a second, separately written
record of the same run, and for `grid` the 34 `followed set changed` lines that admit a
`figure13_newline_patch.exe` name exactly the same 34 PIDs, in the same order, as that
capture's own `summary.per_pid` array. (Checked within one capture deliberately: an earlier
draft of this report cross-referenced `grid`'s `.err` against `midline`'s summary, which are
different runs with disjoint PIDs, and the apparent agreement was an artifact of quoting only
the first 8 lines.) And the no-spill verdict is
corroborated by the experiment's own outcome: the campaign completed without the roughly 15x
slowdown that candle-mi's oracle suite records for a genuine spill (`longrope`, Phi-3.5-mini at
F32, 8,479 MiB growth over 13m18s), so "did not spill" is confirmed by wall-clock as well as by
the instrument.

**One thing I got wrong, recorded because the next reader will repeat it.** I first attached
`hmn watch <pid>` to an already-running, already-saturated process and read its `episodes 0, no
spill observed` as contradicting a known spill. It was not: `watch` takes its baseline from the
first sample, so growth that happened before the attach is baseline, not an episode. The tool
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
- candle-mi's `scripts/resurrect.ps1`, for the contrasting integration: `hmn spill --json`
  wrapped around oracle entries marked `Spill = $true`, which records no process names and so
  needs no filter.
