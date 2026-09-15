# `hypomnesis` v0.2.11 — roadmap

> *`hmn ps` learns to say "spilling" instead of making the operator infer it.*

**Status: ✅ shipped 2026-09-15.**

---

## Why v0.2.11 (and not v0.3.0)

All four changes below are **additive, CLI-only, and backward-compatible**: a
new JSON field / text column on `hmn ps` (`spilling`, defaulting to present
on every row — no flag needed, matching how `shared_used_bytes` and
`driver_version` were added in earlier releases), a new field on
`hmn watch --json` samples (`wall_clock`), a new opt-in `hmn ps --min <SIZE>`
flag, and a brand-new `hmn fits <SIZE>` subcommand. No pre-v0.2.11 output
shape is removed or renamed, no default behavior changes for a caller who
doesn't reach for the new surface. No library-surface change beyond one new
public function (`hypomnesis::snapshot_is_spilling`) — additive under the
same reasoning `#[non_exhaustive]` already gives every other public type in
this crate. Confirmed against the crate's own six-release history of
shipping this exact shape of change under a patch-position bump
(v0.2.4 → v0.2.10) before starting work, not assumed.

## Origin — a `candle-mi` dogfooding report, two campaigns in one file

[`docs/dogfooding-feedbacks/dogfooding-orphan-attribution-and-ps-spill-flag.md`](dogfooding-feedbacks/dogfooding-orphan-attribution-and-ps-spill-flag.md)
(2026-09-14) covers two GPU campaigns on the same machine a day apart.
Campaign 1 is the field validation the v0.2.7 report asked for:
`hmn watch --follow-new` stood guard over 36 sequential short-lived
processes, 360 samples, zero false positives — closing that request's own
loop. Campaign 2 is the one that drove this release: a prompt took 881 s
where the design predicted ~120 s, and `hmn ps`'s SHARED column is the only
reason the cause was found at all — two orphaned test binaries holding
8.6 GiB, the real job spilling 6.8 GiB into shared memory. `nvidia-smi`
showed a full card and nothing else; `hmn ps` named the culprit in one
command. The gap the report names precisely: `hmn watch` computes a
`spilling` boolean per sample, but `hmn ps` — the command anyone reaches for
*first* — carried no such signal at all, so the operator had the two numbers
that imply spill side by side and still had to reason to the verdict by eye.

Four requests, in the report's own priority order (the same order
implemented and shipped in): a spill indicator on `hmn ps` rows; a wall-clock
timestamp on `hmn watch --json` samples, to join a spill trace against a
training driver's own run log without hand-converting `t_ms` offsets; a
`hmn ps --min <SIZE>` filter, to stop piping through `awk` for "who is
actually holding this card"; and a headroom predicate, `hmn fits <SIZE>`,
because every long GPU run in the project was about to hand-roll that exact
check before launching.

## Scope

1. **`hmn ps` SPILL column / `spilling` field.** `hmn ps` is a single
   snapshot with no time series, so it cannot check "has shared-resident
   *grown* above its baseline" the way `hmn watch`'s co-condition does. The
   new `hypomnesis::snapshot_is_spilling(device_index) -> Option<bool>`
   instead takes one live adapter-wide `PDH` sample (the same source
   `SpillTracker` already uses) and applies the same two thresholds with the
   growth check replaced by an absolute floor — sampled *before* the
   process listing, not after, so the verdict and a row's SHARED figure
   describe the same instant. Computed once per device, broadcast to every
   row on it (mirroring `hmn watch`'s existing broadcast shape).
   `null`/`?` — never `false`/`no` — when spill can't be assessed at all,
   including the narrower case where the adapter's own dedicated capacity
   comes back unassessable; `hmn watch`'s own `spilling` field picked up the
   identical `Option<bool>` honesty in the same release, so the two
   commands never disagree about what "can't tell" looks like.
2. **`hmn watch --json` `wall_clock` field.** Absolute UTC ISO-8601 with
   millisecond precision, alongside the existing relative `t_ms`, captured
   at the same instant as `t_ms`'s own reference point (including the first
   sample, where that reference point sits *before* the attach-time setup
   work — `SpillTracker::new`'s `PDH` enumeration among it — not after).
   Formatted by a new, dependency-free `iso8601_utc_millis`/
   `civil_from_days` pair (proleptic-Gregorian civil-from-days integer
   arithmetic, Howard Hinnant's public-domain algorithm) rather than a
   `chrono`/`time` crate, matching the project's minimal-dependency ethos
   for a CLI-only addition.
3. **`hmn ps --min <SIZE>`.** Hides rows below a *total* footprint
   (`used_bytes + shared_used_bytes`, matching `--sort total`'s
   definition — "who is actually holding this card" — not dedicated
   alone). A new shared `parse_size_bytes` parser, modeled on the existing
   `parse_duration`, accepts a bare byte count or a decimal number with
   `KiB`/`MiB`/`GiB` — the same spellings `hmn` itself prints, including the
   space `format_vram` always puts before the unit — so a value copied
   straight out of `hmn`'s own output parses back in. Also backs
   `hmn fits`'s `SIZE` argument.
4. **`hmn fits <SIZE>`.** Exits `0` if `SIZE` fits in the target device's
   current free VRAM, `1` if it doesn't, `2` on a hard error — deliberately
   parallel to `hmn watch`'s `0`/`1`/`2` contract, gateable from a run
   script instead of a hand-rolled `hmn --json | jq` check per launcher.
   The message always states an exact headroom/shortfall margin, not just
   the two rounded `free`/`requested` figures, so a near-miss where both
   round to the same displayed string still reads unambiguously.

## Verification — two independent review passes, not one

Every feature was live-verified on the reference RTX 5060 Ti as it shipped
(each has its own commit; see the per-feature commits `7efd530`, `7e0aee7`,
`42e5e0d`, `90595f7`). Two further, independent code-review passes then ran
over the whole diff before this release was called done:

- **Pass 1** (high effort) found four real issues: `snapshot_is_spilling`
  paying for a live `PDH` sample even when a device's rows were about to be
  discarded by `gpu_processes`'s own failure path; the same call sampled
  *after* `gpu_processes()` rather than before; `hmn watch`'s `t_ms`/
  `wall_clock` pairing straddling the interval query's own duration; and
  the `fold`/`saturated_with_shared_floor` dedicated-threshold arithmetic
  duplicated with no shared source of truth. All four fixed (commit
  `ba0e203`).
- **Pass 2** (max effort, nine parallel review angles, explicitly briefed to
  ignore the first pass's conclusions and start fresh) found a further,
  larger batch, the most significant being that `snapshot_is_spilling` and
  `hmn watch`'s own `spilling` field could still collapse "unassessable" into
  `Some(false)`/`false` in one narrow case each — the exact "measured, not
  spilling" misreading this whole release exists to prevent, and the
  single most-repeated finding across all nine angles. Also: a genuine
  578 ms–1.3 s `wall_clock`/`t_ms` skew on `watch`'s *first* sample (a
  different bug than pass 1's interval-loop fix); a live-reproducible
  self-contradictory `hmn fits` message on a near-miss; `hmn ps --min`'s
  summary line misreporting a real sub-MiB filter as the documented `--min
  0` no-op; and `parse_size_bytes` silently saturating an absurdly large
  value to `u64::MAX` instead of rejecting it (its own justifying comment
  was factually wrong about why that couldn't happen). All fixed (commit
  `7fb68fc`), each with a live reproduction before the fix and a live
  re-verification after.

Architectural findings from the same passes — unifying `fold` and
`saturated_with_shared_floor` behind one `spill_condition` core with
pluggable thresholds, a `GpuDeviceInfo::fits()` library method, and a
`PsFilters` struct to replace `run_ps`'s growing positional-parameter list —
were deliberately **not** implemented as part of this release; they're
design decisions, not bugs, and are recorded in
[`ROADMAP.md`](../ROADMAP.md)'s `Speculative: v0.3.0` section, each with its
own un-gating condition, rather than folded into this release unilaterally.

A separate, smaller piece of the same work session: a consistency pass on
[`docs/dogfooding-feedbacks/style_guide.md`](dogfooding-feedbacks/style_guide.md)
against the seven dogfooding reports it describes, fixing two reports whose
`Status` field had never been flipped to "Resolved" despite their requests
shipping, and a self-inconsistent illustrative example in the guide itself
(commit `962bbba`).

## References

- Driving report:
  [`dogfooding-orphan-attribution-and-ps-spill-flag.md`](dogfooding-feedbacks/dogfooding-orphan-attribution-and-ps-spill-flag.md)
  (2026-09-14).
- Predecessor reports it references: `dogfooding-watch-follow-new.md`
  (`--follow-new`, field-validated here), `dogfooding-wddm-spill-detection.md`
  and `dogfooding-spill-triage-watch-mode.md` (the v0.2.5 spill co-condition
  this release's single-snapshot approximation is measured against).
- `CHANGELOG.md`'s `[0.2.11]` entry for the full field-by-field detail.
