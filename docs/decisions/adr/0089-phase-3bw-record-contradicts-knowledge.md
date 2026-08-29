# ADR-0089: Phase 3BW Record Contradicts Knowledge

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed epistemic relationship vocabulary.

## Context

The confirmed relation vocabulary defines `contradicts` in canonical direction
finding/evidence -> claim target. Phase 3BV covered positive Finding-to-Knowledge
validation; the next smallest negative active-Knowledge path is an explicit
Finding-to-Knowledge contradiction relation.

## Decision

1. Phase 3BW extends `RecordKnowledgeRelationCreateOptions` with `contradicts`.
2. Creation requires a Finding Record source and an active Knowledge target at
   the expected branch head.
3. The relation stores canonical type `contradicts` in the existing relation
   table and WorkState relation membership.
4. `why` reuses the Record-to-Knowledge relation projection and renders this
   edge as `RecordContradicts`.
5. The CLI exposes `record link-contradicts-knowledge`.
6. This slice does not change Knowledge status automatically, does not implement
   Evidence endpoints, and does not add automatic conflict resolution.

## Consequences

- A local Agent can now preserve a Finding that contradicts active Knowledge and
  recover it through `why <knowledge>`.
- Contradiction is modeled as an explicit relation edge, not as a lifecycle
  transition.

## Implementation Findings

- Active Knowledge can be contradicted without becoming invalidated in the
  current state model.
