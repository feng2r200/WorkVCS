# ADR-0108: Phase 4E Merge Item Resolution Runtime

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge lifecycle sequence after ADR-0107.

## Context

Phase 4D persists deterministic `merge_item` rows for the differences captured
when a merge attempt starts. The next tool-facing step is to let a user or
runtime process record tentative resolutions for those items without freezing
the final resolution set or creating a merge commit.

## Decision

1. Phase 4E adds `MergeResolutionKind` with `ours`, `theirs`, and `custom`.
2. `MergeResolveOptions` writes or updates one row in
   `merge_resolution_runtime` for a single `merge_item_id`.
3. `ours` and `theirs` resolutions must not include `custom_payload_json`.
4. `custom` resolutions must include canonical object `custom_payload_json`.
5. Every runtime resolution stores canonical object `rationale_json`, optional
   `resolved_by_session_id`, and `resolved_at_us`.
6. Resolution writes require the owning merge attempt to still be active.
7. If a session is provided, it must belong to the same workspace as the merge
   attempt and its activity timestamp is updated.
8. `MergeItemSnapshot` now carries optional runtime resolution state, and CLI
   `merge resolve` writes it through the Engine facade.
9. This slice does not write `merge_resolution`, freeze resolutions, continue a
   merge, create a merge commit, advance a Branch head, or resolve semantic
   conflicts automatically.

## Consequences

- Operators can revise tentative item choices by issuing another resolution
  write for the same item.
- Later continue/freeze slices can consume `merge_resolution_runtime` as the
  staging source for immutable `merge_resolution` rows.
- Aborting a merge continues to clear runtime resolution rows for the attempt.

## Implementation Findings

- The schema supports an upsert-style runtime resolution table, so this slice
  did not require any DDL change.
