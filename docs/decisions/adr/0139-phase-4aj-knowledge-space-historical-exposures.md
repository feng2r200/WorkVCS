# ADR-0139: Phase 4AJ KnowledgeSpace Historical Exposures

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0137 and ADR-0138 added direct KnowledgeSpace query sets for current
available and source-stale active Exposures. The confirmed federation model also
requires withdrawn Exposure history to remain queryable rather than being
deleted or hidden.

## Decision

1. Phase 4AJ adds `knowledge_space_historical_exposures` to the Engine facade.
2. Historical Exposures are exactly those in the requested KnowledgeSpace whose
   lifecycle is `withdrawn`, regardless of source-status projection.
3. The query validates that the KnowledgeSpace exists, applies the existing
   deterministic Exposure ordering, and supports a positive limit guard.
4. CLI adds `store knowledge-space-historical-exposures` and renders the
   KnowledgeSpace id plus matching Exposure snapshots.
5. This slice does not implement restoration/reactivation, replacement
   automation, adoption, external source resolution, or cross-Store federation.

## Consequences

- The three confirmed KnowledgeSpace query sets are now directly addressable:
  available current, source-stale active, and historical withdrawn.
- Withdrawing an Exposure keeps its source binding, source-status detail, and
  transition history visible through a first-class read path.

## Implementation Findings

- None.
