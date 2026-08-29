# ADR-0113: Phase 4J Branch Projection Refresh

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge/restore capability sequence after ADR-0112.

## Context

The confirmed versioning design keeps Branch HEAD as the only authoritative
current-state pointer. Current projections are rebuildable acceleration layers:
they may cache the WorkState selected by a Branch head, but they must never
compete with that head or become canonical history.

The schema already contains `branch_projection_state`, `branch_entity_current`,
and `branch_relation_current`. Before this slice, those tables had no real
writer and only appeared as intentionally ignored projection noise in tests.

## Decision

1. Phase 4J adds explicit `Engine::refresh_branch_projection` and
   `Engine::branch_projection` APIs.
2. Refresh resolves the current Branch head, replays its WorkState, verifies the
   replayed digest against the head commit digest, and replaces that Branch's
   current projection rows in one transaction.
3. Refresh writes `branch_projection_state.projection_status = 'complete'`,
   the projected head commit, and the replayed WorkState digest.
4. Refresh writes entity and relation current rows from the replayed WorkState.
   Relation current rows copy relation type, endpoints, and discriminator from
   the authoritative immutable relation identity row.
5. Refresh rejects a Branch head change observed between replay and projection
   replacement.
6. Read reports an effective status:
   `complete`, `not_materialized`, `stale`, or `invalid`.
7. A projection is effective `complete` only when the stored projected commit is
   the current Branch head and the stored projection digest equals the Branch
   head commit digest.
8. The CLI exposes only thin `workvcs projection refresh` and
   `workvcs projection show` wrappers.
9. This slice does not make existing semantic mutations auto-refresh
   projections, does not introduce typed domain projections, and does not change
   replay/query/diff/why authority away from canonical history.

## Consequences

- WorkVCS now has a real materialized current projection foundation for HOT
  Branches.
- Existing reads remain replay-authoritative and continue to ignore projection
  noise unless explicitly reading projection state.
- Later slices can decide where automatic projection maintenance is useful
  without broadening every semantic mutation in this slice.

## Implementation Findings

- `branch_projection_state` intentionally needs an effective `stale` state at
  the API/CLI layer even though the physical schema status vocabulary remains
  `complete`, `not_materialized`, and `invalid`.
