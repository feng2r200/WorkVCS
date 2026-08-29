# ADR-0146: Phase 4AQ Context KnowledgeExposure Provenance

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0143 made KnowledgeExposure adoption create local Knowledge and a provenance
relation. ADR-0144 and ADR-0145 made that relation visible through `why`. The
remaining tool-facing gap is that `context` and `next` summaries still omit
KnowledgeExposure provenance edges for adopted Knowledge.

## Decision

1. Phase 4AQ adds `knowledge_exposure_relations` to `ContextOverview`.
2. The resolver inspects active Knowledge at the active Branch head and includes
   current `knowledge_exposure_derived_from` edges for those Knowledge entities.
3. CLI `context` renders `knowledge_exposure_relations` and
   `context_knowledge_exposure_relation.*` rows.
4. CLI `next` renders `context_knowledge_exposure_relations` as a count.
5. This slice is read-only. It does not change context ranking, budgets,
   runnable selection, adoption mutation, source-stale policy, or federation.

## Consequences

- A resumed Agent can recover adopted Knowledge provenance from context without
  running separate `why` calls first.
- `next` remains a compact workflow summary while exposing whether adopted
  Knowledge provenance is present.

## Implementation Findings

- None.
