# ADR-0102: Phase 3CJ Knowledge Relation Restore

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 3CI Knowledge relation removal boundary.

## Context

Knowledge supersession relations can be removed without deleting their
historical relation version. Tools need the symmetric operation to restore a
previously removed current Knowledge lineage edge when review confirms the edge
should be active again.

## Decision

1. Phase 3CJ adds `KnowledgeRelationRestoreOptions` and
   `KnowledgeRelationRestoreCommit`.
2. Restore requires the relation to be absent from the expected head WorkState
   and the supplied relation version to be an existing semantic
   Knowledge-to-Knowledge supersedes relation.
3. Restore reuses the preserved relation version and writes a normal relation
   membership change with `knowledge.relation.restore` operation/event names.
4. Current list/show/why include the restored relation; historical commits
   before removal remain readable.
5. The CLI exposes `knowledge relation-restore`.
6. This slice does not create a new relation version, add new Knowledge
   relation types, or introduce merge/federation behavior.

## Consequences

- Tools can undo accidental Knowledge lineage removals while retaining
  append-only history.
- Replay continues to apply relation membership creation/removal/restore through
  the existing generic relation operation path.

## Implementation Findings

- The shared relation restore writer now accepts operation and event names so
  Record relation restore and Knowledge relation restore can keep separate
  semantic operation vocabulary without duplicating SQL writes.
