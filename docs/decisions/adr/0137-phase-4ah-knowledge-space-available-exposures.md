# ADR-0137: Phase 4AH KnowledgeSpace Available Exposures

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0134 created Store-local KnowledgeExposure records, ADR-0135 added explicit
withdrawal, and ADR-0136 made local source-status refreshable. The confirmed
federation model requires Knowledge Space queries to distinguish current,
historical, and source-stale Exposure sets while leaving default Context ranking
for source-stale Exposures open.

## Decision

1. Phase 4AH adds `knowledge_space_available_exposures` to the Engine facade.
2. Available Exposures are exactly those in the requested KnowledgeSpace whose
   lifecycle is `active` and whose source-status projection is `current`.
3. The query validates that the KnowledgeSpace exists, applies the existing
   deterministic Exposure ordering, and supports the same positive limit guard as
   Exposure listing.
4. CLI adds `store knowledge-space-available-exposures` and renders the
   KnowledgeSpace id plus the matching Exposure snapshots.
5. This slice does not define source-stale Context suppression or ranking
   policy, automatic refresh scheduling, adoption, replacement automation,
   external source resolution, or cross-Store federation.

## Consequences

- Users and later tools now have a direct read path for the default current
  KnowledgeSpace availability set.
- Historical and source-stale Exposures remain queryable through the broader
  Exposure list filters instead of being hidden or deleted.

## Implementation Findings

- None.
