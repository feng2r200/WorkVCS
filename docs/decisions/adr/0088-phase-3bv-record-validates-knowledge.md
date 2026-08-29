# ADR-0088: Phase 3BV Record Validates Knowledge

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed epistemic relationship vocabulary.

## Context

The confirmed relation vocabulary defines `validates` in canonical direction
finding/evidence -> assumption/knowledge. Phase 3BU covered the negative
Knowledge path with `invalidates`; the next smallest positive path is an
explicit Finding-to-Knowledge validation relation.

## Decision

1. Phase 3BV extends `RecordKnowledgeRelationCreateOptions` with `validates`.
2. Creation requires a Finding Record source and an active Knowledge target at
   the expected branch head.
3. The relation stores canonical type `validates` in the existing relation
   table and WorkState relation membership.
4. `why` reuses the Record-to-Knowledge relation projection and renders this
   edge as `RecordValidates`.
5. The CLI exposes `record link-validates-knowledge`.
6. This slice does not add a new Knowledge lifecycle state, does not implement
   Evidence endpoints, and does not automatically create validation relations
   during Knowledge creation.

## Consequences

- A local Agent can now preserve a Finding that positively validates active
  Knowledge and recover it through `why <knowledge>`.
- `supports` and `validates` remain distinct explicit relation intents even
  when both target active Knowledge.

## Implementation Findings

- Active Knowledge is the correct target precondition for `validates` in the
  current state model because Knowledge has no separate `validated` lifecycle
  state.
