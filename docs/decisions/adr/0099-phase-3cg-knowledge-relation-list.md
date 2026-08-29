# ADR-0099: Phase 3CG Knowledge Relation List

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 3CF Knowledge supersession relation.

## Context

Phase 3CF made Knowledge supersession lineage writable and visible through
`why`, but did not expose a direct read-side listing tool. Agent workflows need
to enumerate current Knowledge-to-Knowledge lineage edges at a branch or commit
and optionally narrow that list by replacement or prior Knowledge.

## Decision

1. Phase 3CG adds `KnowledgeRelationListOptions` and
   `KnowledgeRelationListResult`.
2. `knowledge_relations_at` now returns a typed result anchored to workspace
   and commit.
3. The list supports optional replacement-Knowledge and prior-Knowledge filters.
4. The CLI exposes `knowledge relation-list`.
5. Existing `why` behavior is preserved by reading through the new list result.
6. This slice does not implement relation show/remove/restore and does not add
   new Knowledge relation types.

## Consequences

- Tools can enumerate Knowledge supersession lineage without relying only on
  `why`.
- Future show/remove/restore slices can reuse the same typed list boundary.

## Implementation Findings

- No schema or Replay change was required; Phase 3CF had already introduced the
  `knowledge.relation.create` replay allowance.
