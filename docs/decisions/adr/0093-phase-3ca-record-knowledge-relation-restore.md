# ADR-0093: Phase 3CA Record Knowledge Relation Restore

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and ADR-0092.

## Context

ADR-0092 added Record-to-Knowledge relation removal as a non-destructive
correction path. Since the relation/version rows remain historical data, a local
tool workflow also needs a minimal restore operation for the same edge.

## Decision

1. Phase 3CA adds `RecordKnowledgeRelationRestoreOptions` and
   `RecordKnowledgeRelationRestoreCommit`.
2. Restore requires the branch id, expected head commit id, relation id,
   relation version id, and non-empty rationale.
3. The operation rejects restore when the relation is already present in the
   current WorkState.
4. Restore loads the existing historical Record-to-Knowledge relation version,
   validates that its source and target can still be projected at the expected
   head commit, and writes a normal relation membership change.
5. The CLI exposes `record knowledge-relation-restore`.

## Consequences

- Record-to-Knowledge correction now has create, list, show, remove, and restore
  coverage.
- Restore reuses the original relation version; it does not create a new
  relation version or rewrite history.

## Implementation Findings

- The existing relation membership restore writer is endpoint-agnostic and can
  represent Record-to-Knowledge restore when the restore pre-read uses the
  Record-to-Knowledge projection.
