# ADR-0105: Phase 4B Merge Abort

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge lifecycle sequence after ADR-0104.

## Context

Phase 4A introduced active merge attempts. The frozen domain invariants require
merge to follow `start -> resolve -> continue` or `abort`, and require abort to
leave the target Work State unchanged while preserving merge-attempt provenance.
An active merge also blocks another active merge on the same target Branch, so
the tool needs a minimal close path before classification and continue slices.

## Decision

1. Phase 4B adds a merge-abort operation through the Engine facade.
2. `Engine::abort_merge` validates that the merge attempt exists, still has
   active runtime, and has no immutable outcome yet.
3. Abort writes one `merge_attempt_outcome` row with `outcome = 'aborted'`,
   `result_commit_id = NULL`, and canonical detail JSON.
4. Abort updates `merge_runtime.runtime_json` to lifecycle state `aborted` and
   deletes provisional merge resolution runtime rows for that merge.
5. Abort records one runtime `merge.aborted` event and may attach an active
   same-workspace session as the abort actor.
6. The CLI adds only `workvcs merge abort` as a thin wrapper over the Engine API.
7. This slice does not classify merge items, resolve conflicts, create frozen
   resolutions, create a two-parent commit, update target Branch head, or
   implement federation/remote merge behavior.

## Consequences

- A target Branch can start a replacement merge after an earlier active merge is
  aborted.
- Abort remains metadata/runtime work only; WorkState history and Branch heads
  do not move.
- Later resolution and continue slices can rely on immutable outcome rows to
  distinguish active and closed merge attempts.

## Implementation Findings

- `merge_attempt_outcome` is enough to close active-merge uniqueness without a
  schema change.
- The current implementation has no provisional `merge_resolution_runtime` rows
  yet, but abort already clears that table for the merge so later resolve slices
  can use the same close path.
