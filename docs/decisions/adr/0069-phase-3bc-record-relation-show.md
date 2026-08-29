# ADR-0069: Phase 3BC Record Relation Show

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  existing Record relation list projection.

## Context

Record relation creation returns a stable relation id. `record relation-list`
can inspect current relations, but scripts often need to fetch exactly one
relation by id at a commit, mirroring `record show`.

## Decision

1. Phase 3BC adds `record_relation_at(commit_id, relation_id)` to the core
   history layer.
2. The implementation reuses `record_relations_at`, so it only returns
   relation versions current in the replayed WorkState at that commit.
3. Store and Engine expose the same read-only facade.
4. The CLI exposes `record relation-show STORE --commit ... --relation ...`.
5. This slice does not add relation updates/removal, new relation types, or
   storage-table read access outside the Engine facade.

## Consequences

- Local scripts can inspect one semantic Record relation directly after
  creation or from a saved relation id.
- The command shares the same validation behavior as relation listing.

## Implementation Findings

- The existing list projection was sufficient for exact lookup; no schema or
  replay changes were required.
