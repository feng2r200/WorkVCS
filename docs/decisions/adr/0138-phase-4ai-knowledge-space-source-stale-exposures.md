# ADR-0138: Phase 4AI KnowledgeSpace Source-Stale Exposures

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0137 added the direct KnowledgeSpace available set for Exposures whose
lifecycle is `active` and source-status is `current`. The confirmed federation
model also requires source-stale Exposures to remain visible as a distinct
query set rather than being silently withdrawn or hidden.

## Decision

1. Phase 4AI adds `knowledge_space_source_stale_exposures` to the Engine facade.
2. Source-stale Exposures are exactly those in the requested KnowledgeSpace
   whose lifecycle is `active` and whose source-status projection is `stale`.
3. The query validates that the KnowledgeSpace exists, applies the existing
   deterministic Exposure ordering, and supports a positive limit guard.
4. CLI adds `store knowledge-space-source-stale-exposures` and renders the
   KnowledgeSpace id plus matching Exposure snapshots.
5. This slice does not define source-stale ranking or default Context inclusion,
   automatic source-status refresh, adoption, external source resolution, or
   cross-Store federation.

## Consequences

- Tools can now distinguish available current Exposures from still-active
  source-stale Exposures without scanning broad list filters manually.
- Source drift remains a projection warning; it does not mutate Exposure
  lifecycle.

## Implementation Findings

- None.
