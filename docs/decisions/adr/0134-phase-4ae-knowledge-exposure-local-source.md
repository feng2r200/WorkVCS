# ADR-0134: Phase 4AE KnowledgeExposure Local Source

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0004, ADR-0006, confirmed-state-v0.5, and INV-057 require a
Store-local KnowledgeExposure to bind one exact source KnowledgeVersion through
a KnowledgeSpace. Phase 4AD created KnowledgeSpace identities but deliberately
left Exposure creation outside scope.

## Decision

1. Phase 4AE adds the first KnowledgeExposure vertical slice for local sources.
2. The public Engine facade exposes create/show/list operations:
   `create_local_knowledge_exposure`, `knowledge_exposure`, and
   `knowledge_exposures`.
3. Creating a local Exposure writes `object_identity`, `knowledge_exposure`,
   `knowledge_exposure_local_source`, one initial
   `knowledge_exposure_transition`, `knowledge_exposure_current`, and
   `knowledge_exposure_source_status` in one transaction.
4. The local source binding stores the exact Workspace, Knowledge entity, and
   Knowledge entity version. It never follows source `latest`.
5. The initial lifecycle status is `active`. The initial source status is
   `current` for the resolved local source version.
6. Transition and source-status detail JSON are canonical objects. CLI input is
   parsed through WorkVCS canonical JSON validation.
7. Listing supports filters by KnowledgeSpace, Workspace, source Knowledge,
   lifecycle status, source status, and positive limit.
8. Within one KnowledgeSpace, the same local Workspace/Knowledge/Version tuple
   may only be exposed once by this slice.
9. This slice does not implement external Exposure sources, withdrawal,
   replacement, source-stale refresh, adoption, KnowledgeSpace context ranking,
   cross-Store lookup, network federation, or Bundle import/export of Exposure
   rows.

## Consequences

- A Store can now publish one existing local Knowledge version into a
  KnowledgeSpace without mutating the source Workspace WorkState.
- Later lifecycle and source-status slices can extend the existing transition
  and projection tables without changing the local source binding contract.

## Implementation Findings

- None.
