# ADR-0094: Phase 3CB Context Record Knowledge Relations

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Record-to-Knowledge relation tool slices.

## Context

Context overview already exposes active Knowledge, current Records, and
Record-to-Record relation summaries for local Agent workflows. After
Record-to-Knowledge relation tooling was added, those epistemic edges also need
to be visible in the same context surface.

## Decision

1. Phase 3CB adds `record_knowledge_relations` to `ContextOverview`.
2. The context resolver loads current Record-to-Knowledge relations at the active
   branch head and validates the same workspace/commit anchor as the other
   context projections.
3. The CLI `context` output adds `record_knowledge_relations` and
   `context_record_knowledge_relation.*` rows.
4. The CLI `next` summary adds `context_record_knowledge_relations`.
5. This slice does not change runnable selection, claim behavior, relation
   creation, or relation lifecycle operations.

## Consequences

- Local Agent tools can see Knowledge support, validation, contradiction, and
  invalidation edges when reading context.
- Record-to-Record relation output remains separate from Record-to-Knowledge
  relation output.

## Implementation Findings

- Context overview can reuse the existing Record-to-Knowledge list projection
  without adding SQL-specific context shortcuts.
