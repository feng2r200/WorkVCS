# ADR-0101: Phase 3CI Knowledge Relation Removal

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 3CH Knowledge relation show boundary.

## Context

Knowledge supersession relations can now be created, listed, shown, and
explained. Tools also need a non-destructive way to remove an erroneous current
lineage edge while preserving historical commits where that edge existed.

## Decision

1. Phase 3CI adds `KnowledgeRelationRemoveOptions` and
   `KnowledgeRelationRemoveCommit`.
2. Removal requires the relation to be present in the expected head WorkState
   with the exact expected relation version.
3. Removal writes a normal relation membership change with
   `knowledge.relation.remove` operation/event names.
4. Current list/show/why no longer include the removed relation; historical
   commits before removal remain readable.
5. The CLI exposes `knowledge relation-remove`.
6. This slice does not delete relation rows, implement restore, or add new
   Knowledge relation types.

## Consequences

- Tools can correct mistaken Knowledge supersession lineage without losing
  history.
- Restore can be added as a later slice using the preserved relation version.

## Implementation Findings

- Replay already applies relation membership removals generically; this slice
  only added the Knowledge relation remove operation type to the supported
  normal changeset vocabulary.
