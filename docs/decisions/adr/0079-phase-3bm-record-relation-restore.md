# ADR-0079: Phase 3BM Record Relation Restore

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  implemented Record relation removal slice.

## Context

Phase 3BJ added Record relation removal as a commit-level membership change
that drops a relation from the current WorkState projection without deleting
the historical `relation` or `relation_version` rows. A local Agent also needs
the inverse operation when an edge should become current again after removal.

## Decision

1. Add a semantic Record relation restore operation:
   `record.relation.restore` schema version `1`.
2. Restore reinstalls an existing `relation_id` and `relation_version_id` into
   the current WorkState. It does not create a new relation identity or a new
   relation version.
3. Restore is represented as a normal commit with one
   `relation_membership_change` row whose `before_relation_version_id` is
   `NULL` and whose `after_relation_version_id` is the restored relation
   version.
4. Restore rejects a relation that is already current at the expected head.
5. Restore validates that the relation is a semantic Record relation and that
   its endpoints are present and projection-valid at the expected head.
6. The CLI exposes the operation as `record relation-restore`.

## Consequences

- A local Agent can undo a relation removal without minting a second relation
  identity for the same historical edge.
- Replay uses the same membership rule as creation: absent before, present
  after.
- Relation version updates and new identity creation after logical-key removal
  remain deferred.

## Implementation Findings

- The Phase 3BJ replay extension already supports the required membership
  transition shape. Restore only needed a new operation type and write path.
