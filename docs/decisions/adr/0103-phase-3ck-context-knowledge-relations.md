# ADR-0103: Phase 3CK Context Knowledge Relations

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Knowledge relation tool slices through Phase 3CJ.

## Context

Knowledge-to-Knowledge supersession relations can now be created, listed, shown,
removed, and restored. Context overview already exposes active Knowledge and
Record-to-Knowledge relations, but it does not yet surface current Knowledge
lineage edges in the same session context output.

## Decision

1. Phase 3CK adds `knowledge_relations` to `ContextOverview`.
2. The context resolver loads current Knowledge-to-Knowledge relations at the
   active Branch head and validates the same workspace/commit anchor as the
   other context projections.
3. The CLI `context` output adds `knowledge_relations` and
   `context_knowledge_relation.*` rows.
4. The CLI `next` summary adds `context_knowledge_relations`.
5. This slice does not change runnable selection, claim behavior, relation
   lifecycle operations, context ranking, profile budgets, or Knowledge
   federation behavior.

## Consequences

- Local Agent tools can see active Knowledge lineage while reading context.
- Superseded prior Knowledge can stay outside the active Knowledge summary while
  the current supersession relation remains visible as a relation edge.

## Implementation Findings

- Context overview can reuse the existing Knowledge relation list projection;
  no SQL-specific context shortcut or new replay behavior was required.
