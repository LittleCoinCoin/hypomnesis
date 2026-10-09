# A10 docs: the fifth behaviour change in the CHANGELOG and R01

**Goal**: Document what the review found undocumented:
- an unresolved `graphics_footprint` template gives `NoGpuSource` and `hmn ps` exit `2`;
- `gpu_processes` returns `ProcessListDenied` where it returned `NoGpuSource` or an empty list;
- the `Failed` outcome review_code adds;
- the corrected byte-identical and "same pass" sentences in R01.

Each edit is a `git commit --fixup=<owning sha>` on top of 1c97d88. The coordinator writes this leaf, and the user reviews the prose before the push.

**Pre-conditions**:
- [ ] Each step's **Commit** names its owning commit's subject; edits for another owner are listed in the step, one `fixup!` each.
- [ ] Amendment A10 approved (2026-10-09), source `__reports__/v0214_roadmap/17-amendment_a10_pr_c_review_v0.md`.
- [ ] Branch `task/a10_docs` at 1c97d88.

**Success Gates**:
- ⬜ [static] **Five behaviour changes.**
  - `grep -c 'Four behaviour changes are deliberate' docs/roadmap-v0.2.14.md` — before: 1 → after: 0.
  - `grep -c 'Five behaviour changes are deliberate'` — before: 0 → after: 1.
  - The fifth item names `NoGpuSource`, the template index and `hmn ps` exiting `2`.
  - R01's "the fourth deliberate behaviour change" (~429) stays true: the new item comes last.
  - Catches: the change left out of R01's list.
- ⬜ [static] **R01 sentences.**
  - `grep -c 'in the same pass'` — before: 1 → after: 0.
  - `grep -c 'four outcomes'` — before: 1 → after: 0; `grep -c 'five outcomes'` — after: 1.
  - `grep -c 'four-outcome'` — before: 1 → after: 0.
  - The byte-identical sentence (~127) names the unresolved-template exception.
  - Catches: R01 sentences that are false for the final tree.
- ⬜ [static] **CHANGELOG *Changed*.**
  - Between `### Changed` and `### Fixed` of `[Unreleased]`: `grep -c 'ProcessListDenied'` — before: 0 → after: ≥ 1; `grep -c 'NoGpuSource'` — after: ≥ 1.
  - Between `### Added` and `### Changed`: `grep -c 'where it returned .NoGpuSource.'` — before: 1 → after: 0, so nothing is said twice.
  - `grep -c 'bytes, denied or gone' CHANGELOG.md` — before: 1 → after: 0.
  - Catches: a `NoGpuSource` matcher finding nothing under *Changed*.
- ⬜ [static] (guard) **Writing rules.** The added lines (`git diff 1c97d88 -U0 -- CHANGELOG.md docs/roadmap-v0.2.14.md | grep '^+'`) contain no `\bnow\b`, `no longer`, `unchanged` or `not new`.
- ⬜ [run] (guard) **Trial autosquash.** `GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash b716087` on a scratch branch exits 0, or each conflict is resolved to the final text and listed. The known one: 1c97d88's edit to R01 line 157 against the 9bf9ab0 rewrite of the outcomes paragraph; keep the 9bf9ab0 text.

**References**: [PR C review](https://github.com/mi-for-the-rust-of-us/hypomnesis/pull/8#pullrequestreview-5453310396), "Requested changes" 1 and the "in the same pass" nit; the CHANGELOG *Changed* voice of 0.2.13 and 0.2.12 (a bold lead, the files in parentheses, then "— … where it returned …")

## Step 1: The CHANGELOG states the changed errors
**Goal**: A library user who matches on `NoGpuSource` finds both changes under *Changed*.
**Implementation Logic**:
1. Owner d774660. Append one *Changed* bullet after the last one, before `### Fixed`:
   > - **On macOS, `gpu_processes` returns an error where it returned an empty list** (`src/gpu/metal.rs`, `src/gpu/mod.rs`, `src/error.rs`) — when the `graphics_footprint` entry of the ledger template does not resolve, or when no ledger read succeeds and none is refused, it returns `NoGpuSource`, where it returned an empty list; `hmn ps` then exits `2`, where it printed `0 GPU processes found.` and exited `0`, unsandboxed included. When the process list is enumerated but no process other than the caller's can be read, it returns `ProcessListDenied`, where it returned `NoGpuSource` or, under a sandbox that denies only the ledger read, an empty list. `gpu_process_listing` returns the same errors. `HypomnesisError` is `#[non_exhaustive]`, so a `match` on it has a wildcard arm.
2. Also in d774660, trim the *Added* bullet's "`gpu_processes` keeps its signature and returns the same error, where it returned … an empty list;" to "`gpu_processes` keeps its signature and returns the same error;". In the same bullet, say that `denied_pids` is sorted by `pid`.
3. Owner 9bf9ab0. Line ~13, "A per-PID ledger read is bytes, denied or gone", becomes "A per-PID ledger read is bytes, denied, gone (`ESRCH`) or failed".
**Deliverables**: `CHANGELOG.md` (the *Changed* bullet, the *Added* trim, the outcomes sentence)
**Consistency Checks**: `bash -c 'test "$(grep -c "bytes, denied or gone" CHANGELOG.md)" = 0 && test "$(sed -n "/^### Changed/,/^### Fixed/p" CHANGELOG.md | head -200 | grep -c ProcessListDenied)" -ge 1'` (expected: PASS)
**Commit**: `docs(gpu): document gpu_process_listing, its denied PIDs and ProcessListDenied`

## Step 2: R01 lists five behaviour changes and states the read as it is
**Goal**: R01 is true for the final tree.
**Implementation Logic**:
Owner 9bf9ab0 unless noted.
1. Line ~26: "Four" becomes "Five". Item 4's closing "." becomes ";", and this item is appended:
   > - `gpu_processes` returns `NoGpuSource`, not an empty list, when the `graphics_footprint` template index does not resolve, so `hmn ps` exits `2` on such a host, unsandboxed included, where v0.2.13 prints `0 GPU processes found.` and exits `0`.
2. Line ~125-127: "…keeps byte-identical output." becomes "…keeps byte-identical output, except on a host whose ledger template lacks the `graphics_footprint` entry, which the fifth deliberate change in *Why v0.2.14* covers."
3. Line ~129-131: "…and takes `p_comm` from the record, so the enumeration fallback gets names in the same pass." becomes "…and takes `p_comm` from the record. The enumeration fallback keeps only each record's PID; a name is read per row, from `proc_pidpath` and, where that is refused, from the `p_comm` of the PID's own `KERN_PROC_PID` record."
4. Lines ~156-160 become:
   > - **A per-PID read has five outcomes, not two.** `read_graphics_footprint` stops folding everything into `None`. It returns bytes; *denied* (`EPERM`); *gone* (`ESRCH`); *failed*, for any other errno or a reply it cannot use; or *unavailable*, when the `graphics_footprint` template index did not resolve. *Unavailable*, both enumerations refused, or a listing where no read worked, none was refused and at least one failed, makes the backend return `None`. The dispatcher then falls through to `NoGpuSource`, as for every other backend, instead of today's silent empty list.

   This absorbs 1c97d88's line-157 edit ("or any other errno").
5. Owner 6e7cf09. Scope row 3 (~257): "four-outcome ledger read" becomes "five-outcome ledger read".
**Deliverables**: `docs/roadmap-v0.2.14.md` (the list, the two sentences, the outcomes paragraph, scope row 3)
**Consistency Checks**: `bash -c 'R=docs/roadmap-v0.2.14.md; test "$(grep -c "Five behaviour changes are deliberate" $R)" = 1 && test "$(grep -c "in the same pass" $R)" = 0 && test "$(grep -c "four-outcome" $R)" = 0 && test "$(grep -c "four outcomes" $R)" = 0'` (expected: PASS)
**Commit**: `docs(metal): say a p_comm name is cut at 16 bytes and filters cannot match past it`
