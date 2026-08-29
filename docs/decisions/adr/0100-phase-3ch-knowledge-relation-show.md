# ADR-0100: Phase 3CH Knowledge Relation Show

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 3CG Knowledge relation list boundary.

## Context

Phase 3CG exposed Knowledge supersession relation listing. Tools also need a
stable single-relation read path for inspecting a specific lineage edge by
relation id at a branch or commit.

## Decision

1. Phase 3CH adds `knowledge_relation_at`.
2. The lookup resolves through the typed Knowledge relation list boundary and
   returns `KnowledgeRelationSnapshot`.
3. The CLI exposes `knowledge relation-show`.
4. The command accepts the same branch-or-commit target pattern as other
   historical read commands.
5. This slice does not implement relation removal, restore, or new relation
   types.

## Consequences

- Tools can inspect a single Knowledge supersession edge without parsing a full
  relation list.
- Future removal and restore commands can use the same snapshot fields as their
  expected-version inputs.

## Implementation Findings

- No new storage or Replay behavior was required.
