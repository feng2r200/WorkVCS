# ADR-0107: Phase 4D Merge Item Classification

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge lifecycle sequence after ADR-0104 through ADR-0106.

## Context

Merge attempts can now be started, aborted, shown, and listed, but a started
merge still has no persisted queue of the WorkState differences that later
resolve and continue operations should process. The schema already provides
`merge_item`; this slice needs to populate it without opening merge resolution
or merge commit creation.

## Decision

1. `Engine::start_merge` classifies WorkState entity and relation membership
   differences between the merge base, captured target head, and captured source
   head.
2. Classification is deterministic: entity subjects are considered first,
   relation subjects second, and subjects are ordered by their typed ids.
3. A subject whose target and source versions are identical produces no item.
4. A source-side change where target still equals base produces an `AUTO` item.
5. A target-side-only change where source still equals base produces no item for
   this slice.
6. A subject where target and source both differ from base and from each other
   produces a `CONFLICT` item.
7. Each persisted item records a stable ordinal, classification,
   `subject_object_id`, and canonical `item_payload_json` carrying the subject
   id plus base/target/source version ids or `null`.
8. `MergeAttemptSnapshot` now includes ordered `MergeItemSnapshot` values, and
   CLI `merge show`/`merge list` expose item counts and show item details.
9. This slice does not resolve items, freeze resolutions, continue a merge,
   create a merge commit, advance a Branch head, or replay merge commits.

## Consequences

- Later merge resolution slices can operate from authoritative persisted
  `merge_item` rows rather than recomputing the attempt's initial differences.
- A merge attempt captures both branch heads before classification and confirms
  those heads again inside the write transaction before persisting the attempt.
- Target-only changes are treated as already present on the target side and do
  not create additional work for this initial target-oriented merge queue.

## Implementation Findings

- Merge item classification depends on `history::state_at` for the captured
  base, target, and source commits. Since merge commit replay is still deferred,
  starting a merge from a branch head that is itself an unreplayable merge commit
  remains outside this slice.
