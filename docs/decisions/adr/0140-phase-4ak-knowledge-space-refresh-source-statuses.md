# ADR-0140: Phase 4AK KnowledgeSpace Source-Status Batch Refresh

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0136 added single-Exposure local source-status refresh. ADR-0137,
ADR-0138, and ADR-0139 then exposed the main KnowledgeSpace read sets for
available current, active source-stale, and withdrawn historical Exposures. A
tooling workflow still needs a first-class way to refresh a KnowledgeSpace
before reading those sets, without requiring callers to enumerate every active
Exposure manually.

## Decision

1. Phase 4AK adds `refresh_knowledge_space_source_statuses` to the Engine
   facade.
2. Batch refresh is limited to Exposures in the requested KnowledgeSpace whose
   lifecycle is `active`.
3. The slice reuses the existing single-Exposure local source-status refresh
   semantics and does not duplicate or redefine source drift detection.
4. The result returns the requested KnowledgeSpace id, refreshed Exposure
   snapshots, and counts for `current`, `stale`, `unknown`, and `unresolved`.
5. CLI adds `store knowledge-space-refresh-source-statuses` and renders the
   summary counts plus refreshed Exposure snapshots.
6. This slice does not implement scheduling, replacement automation, adoption,
   external source resolution, cross-Store federation, or lifecycle changes for
   Exposures.

## Consequences

- Callers can refresh a KnowledgeSpace and then query available/source-stale
  sets without manually looping over Exposure ids.
- Withdrawn Exposures remain historical records and are not mutated by the
  batch refresh operation.

## Implementation Findings

- None.
