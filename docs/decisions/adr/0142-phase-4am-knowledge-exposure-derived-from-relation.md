# ADR-0142: Phase 4AM Knowledge Derived From KnowledgeExposure Relation

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0141 created a read-only adoption candidate. The confirmed domain model
requires explicit Knowledge adoption to retain source-version provenance and to
create a `derived_from -> KnowledgeExposure` edge. The physical schema already
allows relation endpoints to reference any `object_identity`, including
`knowledge_exposure`, while existing high-level helpers covered only narrower
Record/Knowledge relation families.

## Decision

1. Phase 4AM adds a narrow Engine facade for creating a `derived_from` relation
   from an active Workspace-local Knowledge entity to an active
   KnowledgeExposure.
2. The relation is written into the target Workspace Work-State as a normal
   relation membership change.
3. The changeset reuses the already replay-supported
   `knowledge.relation.create` operation type and schema version.
4. The helper validates branch head, Workspace membership for the source
   Knowledge, active Knowledge status, active Exposure lifecycle, and duplicate
   `derived_from` edges for the same Knowledge/Exposure pair.
5. CLI adds `store knowledge-exposure-derived-from-link`.
6. This slice does not create adopted Knowledge, copy source Knowledge state,
   choose source-stale adoption policy, create replacement semantics, resolve
   external sources, or implement cross-Store federation.

## Consequences

- The next adoption mutation can create Workspace-local Knowledge and then link
  it to the source Exposure without adding another relation persistence path.
- Replay remains within already supported normal relation membership semantics.

## Implementation Findings

- None.
