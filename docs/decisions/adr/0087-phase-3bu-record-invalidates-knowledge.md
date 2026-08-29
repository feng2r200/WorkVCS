# ADR-0087: Phase 3BU Record Invalidates Knowledge

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed epistemic relationship vocabulary.

## Context

Knowledge already has an explicit invalidation transition. The relation
vocabulary also defines `invalidates` in canonical direction
finding/evidence -> assumption/knowledge. After Phase 3BT added
Record-to-Knowledge support edges, the next smallest tool slice is preserving
the explicit Finding that invalidated a Knowledge statement.

## Decision

1. Phase 3BU extends `RecordKnowledgeRelationCreateOptions` with
   `invalidates`.
2. Creation requires a Finding Record source and an already invalidated
   Knowledge target at the expected branch head.
3. The relation stores canonical type `invalidates` in the existing relation
   table and WorkState relation membership.
4. `why` reuses the Record-to-Knowledge relation projection and renders this
   edge as `RecordInvalidates`.
5. The CLI exposes `record link-invalidates-knowledge`.
6. This slice does not automatically create the relation during
   `knowledge invalidate`, does not support Evidence endpoints, and does not
   implement Knowledge `validates`, `contradicts`, `supersedes`, or adoption.

## Consequences

- A local Agent can now preserve the explicit Finding behind an invalidated
  Knowledge statement and recover it through `why <knowledge>`.
- Knowledge invalidation remains a separate semantic transition, keeping the
  atomic combined operation for a later explicit slice if needed.

## Implementation Findings

- The Phase 3BT Record-to-Knowledge relation projection generalized cleanly
  from `supports` to `supports | invalidates`; replay and schema did not need
  changes.
