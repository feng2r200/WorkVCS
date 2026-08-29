# ADR-0114: Phase 4K Projection Invalidation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 projection sequence after ADR-0113.

## Context

ADR-0113 introduced explicit Branch projection refresh and read APIs. A complete
projection is only current when it binds the Branch HEAD commit and digest. After
semantic operations move a Branch HEAD, leaving older current-projection rows in
place is still detectable as stale, but it is less useful for tools than an
explicit not-materialized marker.

## Decision

1. Phase 4K adds one internal helper that marks a Branch projection
   `not_materialized`.
2. The helper clears `branch_entity_current` and `branch_relation_current` rows
   for the Branch, then upserts `branch_projection_state` with null projected
   commit and digest in the same transaction.
3. EntityTransition, Task/Acceptance/Verification semantic writes, Record and
   Knowledge relation writes, Structural Reference writes, Primary Containment
   writes, WorkState restore, and merge continue call this helper after a
   successful Branch HEAD compare-and-swap.
4. Projection invalidation creates no Event, ChangeSet, ChangeOperation,
   WorkStateCommit, or runtime row.
5. This slice does not auto-refresh projections, materialize typed domain
   projections, or make replay/query/diff/why depend on projection tables.

## Consequences

- A moved Branch no longer retains stale entity/relation current rows after a
  successful semantic write.
- Tools can distinguish "needs refresh" from a legacy/corrupt stale projection
  without replaying the Branch.
- Branch HEAD remains the sole current-state pointer.

## Implementation Findings

- The 4J effective `stale` status remains useful for legacy or manually
  corrupted projection rows, but normal Engine-owned writes now move from
  `complete` to `not_materialized`.
