# ADR-0092: Phase 3BZ Record Knowledge Relation Removal

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and ADR-0090/0091.

## Context

Record-to-Knowledge relation creation, list, show, and `why` explanation are now
available. Local tool workflows also need a minimal correction path when a
Knowledge relation edge was created by mistake.

## Decision

1. Phase 3BZ adds `RecordKnowledgeRelationRemoveOptions` and
   `RecordKnowledgeRelationRemoveCommit`.
2. Removal requires the branch id, expected head commit id, relation id,
   expected relation version id, and non-empty rationale.
3. The operation verifies that the current WorkState contains the exact
   Record-to-Knowledge relation version before removal.
4. Removal writes a normal relation membership change with
   `after_relation_version_id = NULL`.
5. The relation/version rows remain historical data and can still be queried at
   commits before removal.
6. The CLI exposes `record knowledge-relation-remove`.

## Consequences

- Tools can correct a mistaken Record-to-Knowledge edge without deleting
  historical provenance.
- Restore remains out of scope for this slice.

## Implementation Findings

- The existing relation membership removal writer is endpoint-agnostic and can
  safely represent Record-to-Knowledge removal when the pre-read uses the
  Record-to-Knowledge projection.
