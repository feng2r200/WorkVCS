# ADR-0136: Phase 4AG KnowledgeExposure Source Status Refresh

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0134 introduced local-source KnowledgeExposure rows and initialized their
source-status projection as `current`. ADR-0135 added Exposure withdrawal while
preserving the source binding and source-status row. The next required slice is
to make the source-status projection refreshable against current local WorkVCS
history without introducing external source resolution.

## Decision

1. Phase 4AG adds `refresh_knowledge_exposure_source_status` to the Engine
   facade.
2. Refresh is limited to local Exposure sources. It reads the source workspace's
   branch heads, replays each head, and compares the Exposure's bound
   Knowledge entity/version against the replayed WorkState.
3. The refreshed status is:
   - `current` when at least one checked branch head still contains the exact
     bound Knowledge entity version;
   - `stale` when no checked head matches and at least one checked head contains
     either a different version of the same Knowledge entity or no such entity;
   - `unknown` when no branch heads are available to check.
4. Refresh updates the existing `knowledge_exposure_source_status` row with
   `source_status`, `checked_at_us`, and canonical object detail JSON containing
   checked, matching, drifted, and missing branch-head counts.
5. Refresh does not change Exposure lifecycle, transition history, local source
   binding, or the original source Knowledge state digest.
6. CLI adds `store knowledge-exposure-refresh-source-status` and renders the
   same KnowledgeExposure snapshot shape used by create/show/withdraw/list.
7. This slice does not implement external sources, unresolved-source probing,
   adoption, automatic refresh scheduling, replacement semantics, or Bundle
   transport for Exposure rows.

## Consequences

- Local KnowledgeSpaces can now detect when a published Exposure's bound source
  Knowledge version has drifted from the source workspace's current branch
  heads.
- Source-status detail remains deterministic canonical JSON and can be used by
  later projections without reinterpreting CLI text output.

## Implementation Findings

- None.
