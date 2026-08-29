# ADR-0076: Phase 3BJ Record Relation Removal

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  implemented Record relation slices.

## Context

Record relation slices can now create typed causal and semantic Record edges
and query them from a replayed WorkState. A local Agent also needs to withdraw
an edge from the current projection when a relation no longer describes the
current semantic state, while preserving the historical commit where that
relation existed.

## Decision

1. Add a semantic Record relation removal operation:
   `record.relation.remove` schema version `1`.
2. Relation removal is represented as a normal commit with one
   `relation_membership_change` row whose `before_relation_version_id` is the
   current relation version and whose `after_relation_version_id` is `NULL`.
3. The physical `relation` and `relation_version` rows are not deleted.
4. Replay applies relation removal by requiring the current WorkState relation
   version to match `before_relation_version_id`, then removing that relation
   from the replayed relation mapping.
5. `record_relations_at` and `record_relation_at` remain commit-relative:
   a removed relation is absent at the removal commit, but still queryable at
   an earlier commit where the relation was current.
6. The CLI exposes the operation as `record relation-remove`.

## Consequences

- Current semantic projections can remove stale Record relation edges without
  losing historical evidence.
- Removal participates in branch-head CAS through the same branch/head
  arguments as relation creation.
- Relation update and relation identity reuse after removal remain deferred.

## Implementation Findings

- Existing replay validation already had the right boundary for relation
  membership changes. The implementation only needed to extend relation replay
  from create-only to create-or-remove.
- Logical-key reuse after removal remains deliberately deferred because the
  current uniqueness check is against persisted relation rows, not only the
  current WorkState projection.
