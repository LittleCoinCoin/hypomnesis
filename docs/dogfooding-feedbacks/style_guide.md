# How to write a dogfooding report

This folder is not a bug tracker and not a wish list. Each file is a **field report from a real
workload**: something used `hypomnesis` in anger, and the report records what the instrument
said, whether it was right, and what would have made it better. That is why these reports have
led to shipped features (`reserved`, `watch`, `--follow-new`, `--sort`, the driver-version
field) rather than to a backlog.

This guide is descriptive: every rule below is already followed by the reports in this folder,
and each names the file it is drawn from. Follow it and a new report will read like a sibling.

---

## The shape

### Title: state the finding, not the topic

    # Dogfooding report (from <consumer>): <what you found>

The `(from ...)` names the workload, because the same tool behaves differently under a trainer,
an oracle suite and a batch of short-lived processes. The rest is a **claim**, not a subject
line. Compare:

> ✅ `hmn watch catches every spill at the adapter — and attributes none of them, because its PID set is frozen at the first sample`
> ❌ `Feedback on watch mode`

A reader scanning the folder should be able to reconstruct the project's history from titles
alone.

### Metadata block: five fields, in this order

    **Date:** 2026-09-14
    **Reporter:** <workload, hardware, OS, hmn version>
    **Severity:** <validation of X> + <request for Y>
    **Affected area:** <the specific surface, not the crate>
    **Status:** Proposed — vX.Y candidate(s)

Then a `---` rule.

`Reporter` carries the hardware and the `hmn` version because every number below depends on
them. `Affected area` names a surface (`gpu::device_info`, `hmn ps` row semantics), never just
"hmn". `Status` is a lifecycle field and is **edited later**: `Proposed — v0.2.7 candidates`
becomes `✅ **Resolved in v0.2.7** (2026-08-02)` when it ships (`dogfooding-watch-follow-new.md`'s
own transition). That editing is what makes the folder a record rather than an archive — a report
whose `Status` line still reads `Proposed` after its request has shipped is stale, not historical.

### Opening: `## TL;DR`

Five to ten lines, and it must contain the verdict and the headline numbers. A reader who stops
here should still learn what happened. The older reports open with `## Context` or `## Summary`
instead; `## TL;DR` is the settled form.

### Body: narrative, then requests

Section headings state conclusions, the way the title does:

> `## What happened — three verdicts in one campaign`
> `## What was missing — the frozen PID set`
> `## Campaign 2 — the failure ps caught, and the step it still leaves to the human`

Paste tool output **verbatim** in fenced blocks, including the columns that make the point, and
annotate inline. `dogfooding-orphan-attribution-and-ps-spill-flag.md` sets `nvidia-smi` output
beside `hmn ps` output for the same moment; the contrast is the argument.

Requests go in a numbered list, **ordered by how much they would have helped**, with a sketch
of the interface where one is obvious (`hmn watch [PID ...] [--interval 30s]`). Say which single
change matters most; a list of equals is a list nobody can schedule.

### Closing sections, in this order

| Section | Purpose | Required |
|---|---|---|
| `## Smaller observations` | numbered, one paragraph each; the things too small for their own section | when there are any |
| `## Acceptance fixtures (already run, free to regress against)` | a table of the cases you exercised, with the expected verdict | when the report claims a behaviour |
| `## Confidence` | how sure you are, and what independently corroborated it | always |
| `## References` | predecessor reports, the semantics under test, the consumer's own artifacts | always |

The fixtures table is the highest-leverage part: it converts a report into regression cases the
crate can be tested against. Give it real measured columns, not invented ones.

---

## The standards that make these reports work

**Every claim carries a measurement with units.** Not "it was slow" but "881 s where the design
predicted 120 s". Not "shared memory grew" but "142 → 718 MiB against a 14.9 GiB dedicated".
Numbers make a report checkable later, which is the only reason old reports still have value.

**Corroborate with an independent signal.** A tool cannot validate itself. The spill-triage
report used epoch pace recovered from checkpoint file mtimes; the orphan report used wall-clock
per run from the experiment's own log. Say in `## Confidence` what the second signal was. A
verdict confirmed by its cure ("the fix restored 119 s/epoch") is the strongest form available.

**Report what worked, not only what failed.** `dogfooding-install-no-binary-and-protected-names.md`
devotes a numbered section to `watch` deciding a real question in one command, in a report
otherwise about an install bug. Knowing which parts are load-bearing tells the maintainer what
not to break.

**Record your own mistakes.** If you misread the tool, write it down and say the tool was right.
The orphan report records two occasions where its author accused `hmn ps` of listing dead
processes and was wrong both times. This is not self-flagellation: the next person will make the
same mistake against the same rows, and it is often evidence for a genuine ergonomic request.

**Distinguish "the tool is wrong" from "the tool made me work".** Most entries here are the
second kind. `hmn` reporting a long process list is correct by design; needing `awk` to filter
it is the finding. Keep the distinction explicit so a maintainer knows whether behaviour or
affordance is at stake.

**Quantify the benign baseline.** When you observe a normal value, say so numerically, because
it becomes the threshold anchor for the next report: "SHARED at 142–224 MiB against 14.9 GiB
dedicated, roughly 1–1.5%" is what let a later reader recognise 5% and climbing as a leak.

**Aim a request at a version.** `Severity` and the request section both name the next version.
It forces the question "is this the next thing, or the thing after" at writing time.

---

## Mechanics

- Wrap prose at about 100 columns. Titles, the metadata block and table rows may exceed it.
- Markdown links between reports are relative file names, so the folder browses offline.
- File name: `dogfooding-<subject-in-kebab-case>.md`, naming the subject rather than the
  consumer, since one consumer files many reports. (`dogfooding-candle-mi-nvml-reserved.md` is a
  grandfathered pre-guide exception — leave it, don't rename it: it's cross-referenced from two
  other reports and embedded verbatim in every published crates.io package tarball back through
  v0.2.4, so a rename would break links into already-published history for no reader benefit.)
- Em-dashes, arrows and unicode symbols are used freely here; this folder is prose, not a paper.
- Write the report while the evidence is still on screen. Every number in these files was
  pasted, not remembered.
