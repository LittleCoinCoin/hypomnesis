# Dogfooding report (from candle-mi): `--follow-new` passes its field test, and `hmn ps` found an 8× slowdown that nothing else could see — but the operator still has to infer "spilling" by eye

**Date:** 2026-09-14
**Reporter:** candle-mi Figure-13 poetry-planning experiments (composition-horizon power run + newline activation patching, RTX 5060 Ti 16 GiB, Windows 11 / WDDM, `hmn 0.2.9`)
**Severity:** Field validation of v0.2.7 `watch --follow-new` (36-process run, clean) + four small requests for **v0.2.11**
**Affected area:** `hmn ps` row semantics (no spill flag, no liveness, no size filter); `hmn watch --json` sample schema (no wall-clock stamp)
**Status:** ✅ **Resolved in v0.2.11** (2026-09-15). All four requests shipped: the SPILL column / `spilling` field on `hmn ps`, `wall_clock` on `hmn watch --json` samples, `hmn ps --min`, and `hmn fits`. Smaller observations 3 (a liveness column) and 4 (tenancy in triage) were recorded, not taken up. *(Status flipped 2026-09-26, found stale during v0.2.12's release.)*

---

## TL;DR

Two GPU campaigns on the same machine, a day apart. The first ran clean and is the field
validation `--follow-new` was asking for: **36 sequential short-lived processes, 360 samples,
zero spill episodes**, per-PID rows following the workload instead of freezing on the
compositor. The second went wrong, and `hmn ps` is the only reason we know why: two orphaned
test binaries held **8.6 GiB**, the real job spilled **6.8 GiB into shared memory**, and a
15-minute prompt should have taken under two — an **8× penalty**. `nvidia-smi` showed a full
card and nothing else; the per-process SHARED column named the culprit in one command.

The gap is the last mile of that diagnosis. `watch` computes a `spilling` boolean per sample;
`ps`, which is what anyone runs *first*, does not show one. I had the two numbers that imply
spill side by side and still had to reason my way to the verdict.

## Campaign 1 — `--follow-new` does what the v0.2.7 report asked for

The composition-horizon experiment: 36 sequential `cargo`-built runs, **each a fresh
process**, ~3 hours, on a desktop also running VS Code and a browser. Armed once:

```
hmn watch --follow-new --top 3 --interval 30s --duration 3h --json
```

Result: `observations: 360`, `spilled: false`, `episodes: []`, `peak_dedicated_bytes` 13.69 GiB
against a 15.68 GiB limit, `peak_shared_bytes` 174 MiB against a 159 MiB baseline. The
`per_pid[]` array carried the experiment's own binaries under two different PIDs, exactly the
churn that froze the v0.2.6 auto-selection. **This is the predecessor report's request,
shipped and working**: the instrument followed a spawn-per-run driver without supervision, and
the summary was directly quotable in the paper's methods section ("360 samples, no spill").

One consequence worth stating plainly: because the watch was armed, "the 2.5M-feature
transcoder cell did not spill" is a *measurement* in the write-up rather than an assumption.

## Campaign 2 — the failure `ps` caught, and the step it still leaves to the human

Next day, same machine, an activation-patching run on Gemma 2 2B. Symptom: the first prompt
took **881 s** where the design predicted ~120 s. `nvidia-smi` said only that memory was used:

```
9949 MiB, 6104 MiB          # used, free — true, and diagnostically inert
```

`hmn ps` said what was actually happening:

```
PID    NAME                                        VRAM     SHARED   DEVICE
19792  bench_hook_overhead-49ccba0c81a5a3f1.exe    6.0 GiB  78 MiB   RTX 5060 Ti
12688  bench_hook_diagnostic-27091f79d5092125.exe  2.7 GiB  0 MiB    RTX 5060 Ti
13300  figure13_newline_patch.exe                  5.6 GiB  6.8 GiB  RTX 5060 Ti
```

Three facts in one view, none available from `nvidia-smi` on WDDM: *who* held the card, that
**8.6 GiB of it was two orphaned test binaries** left over from an earlier command, and that
the real job had **6.8 GiB resident in shared memory** — spill, not saturation. Cause
established, cleaned up, rerun: **104 to 119 s per prompt, peak dedicated 12.29 GiB, shared
flat at its 78 MiB baseline.** Gemma at `F32` — the transformer backend loads `DType::F32`
unconditionally — wants 12.3 of the 16.3 GiB card, so the 8.6 GiB of orphans left it no room
at all.

**The last mile.** Row 3 had `VRAM 5.6 GiB` and `SHARED 6.8 GiB` and the adapter was
saturated: by the shipped v0.2.5 co-condition that *is* a spill. I still had to assemble the
verdict myself, from a column whose significance I happened to know. `watch` would have
labelled it; `ps` is the command you reach for when something is slow.

## Requests for v0.2.11, in the order they would have helped

1. **A spill indicator on `ps` rows.** `watch` samples already carry `spilling`; a `ps --json`
   row carries only `pid, name, used_bytes, shared_used_bytes, device_index, device_name`,
   though `ps` is the first-response command. A marker column, or a `ps --spilling` filter,
   would have turned the diagnosis above from a three-step inference into a glance. This is the
   single change that would most improve triage.

2. **A wall-clock stamp in `watch --json` samples.** Fields today are
   `kind, t_ms, pid, name, used_bytes, used_delta_bytes, shared_used_bytes, shared_delta_bytes,
   spilling` — `t_ms` is relative to attach. Correlating a spill trace against the driver's own
   `run.log`, which stamps local time per completed run, had to be done by hand. An ISO-8601
   field per sample makes the join mechanical.

3. **A size filter on `ps`.** Options are `--pid`, `--device`, `--sort`, `--json`; hiding
   desktop noise meant piping through `awk`. A `--min 50MiB` would make "who is actually
   holding this card" a one-liner. (With PDH's every-holder semantics — the right choice, and
   what made the orphans visible — the list is long by design, so the filter is the companion
   to that decision.)

4. **A headroom predicate.** The question that actually mattered was never "what is on the
   GPU" but "will a 12 GiB job fit right now". Something like `hmn fits 12GiB` returning an
   exit code would be gateable from a run script. Every long GPU run in this project will now
   hand-roll that check before launching; a first-class version would be better than six
   copies of it.

## Smaller observations

1. **The help text earned its keep.** The committed-versus-resident explanation is what let me
   read `5.6 GiB dedicated / 6.8 GiB shared` as spill rather than as a contradiction, and the
   note that committed figures can exceed physical VRAM pre-empted a wrong turn. Several of
   this project's own bugs have been misread numbers; this documentation prevented one.

2. **`--json` hygiene is correct and worth guaranteeing.** `hmn ps --json` puts pure JSON on
   stdout and the `hmn: N GPU processes found (X GiB committed total)` summary on stderr, so
   `hmn ps --json | ConvertFrom-Json` works with no stripping. Please keep that contract
   documented; scripts will come to depend on it.

3. **A liveness column would have short-circuited an operator error — mine.** Twice I concluded
   `hmn ps` was listing dead processes, because `Get-Process -Id <pid>` returned nothing for
   rows `hmn` still showed. **`hmn` was right both times**; the processes were alive and my
   single-PID lookup was the unreliable part (the pipeline-and-filter form found them at once).
   `hmn` reported nothing false in either campaign. But an explicit `state` column, or simply
   not having to cross-check, would have stopped me chasing a phantom leak for several minutes.
   Recording it because the next person will make the same mistake against the same rows.

4. **The orphans were not `hmn`'s problem to solve, but they are a recognisable shape.** Two
   short-lived test binaries outlived the shell that spawned them and kept their allocations.
   A `watch` that flagged "a followed PID's parent has exited while it still holds N GiB" would
   be over-engineering; but this is the second report in this folder where *tenants rather than
   the workload* caused the spill (cf. Verdict 3 of
   `dogfooding-spill-triage-watch-mode.md`), which suggests tenancy
   deserves a first-class place in the triage story.

## Acceptance fixtures (already run, free to regress against)

| fixture | dedicated | SHARED | pace | expected verdict |
|---|---|---|---|---|
| C1 horizon, 36 procs, `--follow-new` | peak 13.69 / 15.68 GiB | 174 MiB peak vs 159 baseline | 179 min total | NOT spilling; `per_pid[]` follows the churn |
| C2 orphan contention | 5.6 GiB job + 8.6 GiB orphans | **6.8 GiB** | **881 s/prompt** | SPILL (tenant-driven) |
| C2 after cleanup | peak 12.29 / 16.3 GiB | 78 MiB flat | **104--119 s/prompt (~8×)** | NOT spilling |

## Confidence

High. Both campaigns were cross-validated by an independent signal (wall-clock per run, logged
by the experiment driver), and the spill diagnosis was confirmed by its cure: removing the
orphans restored the predicted pace to within 10% on the first try. `hmn` and `nvidia-smi`
agreed on every device-level figure taken side by side.

## References

- Predecessor: [`dogfooding-watch-follow-new.md`](dogfooding-watch-follow-new.md) — requested
  `watch --follow-new`; Campaign 1 here is its field validation on a 36-process workload.
- Semantics under test: [`dogfooding-wddm-spill-detection.md`](dogfooding-wddm-spill-detection.md)
  (v0.2.5 commit-vs-resident co-condition) and
  [`dogfooding-spill-triage-watch-mode.md`](dogfooding-spill-triage-watch-mode.md) (tenant-driven
  spill, Verdict 3 — the same shape as Campaign 2 here).
- candle-mi: `docs/experiments/figure13-newline/power/` (composition-horizon power run; the
  spill log is committed there as `hmn_watch.jsonl`) and `docs/experiments/figure13-patching/`.
