# ADR-0144: Phase 4AO Why Knowledge Exposure Provenance

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0143 made KnowledgeExposure adoption write a local Knowledge entity and a
`derived_from` relation from that Knowledge to the source Exposure. The
confirmed product definition requires adopted Knowledge provenance to remain
queryable through `why`, but the existing `why` model only rendered Entity and
Evidence endpoints.

## Decision

1. Phase 4AO extends `why` relation endpoints with a KnowledgeExposure endpoint.
2. `why` now includes current WorkState relations where:
   - `relation_type = derived_from`;
   - source endpoint is a Workspace-local Knowledge entity;
   - target endpoint is a KnowledgeExposure object.
3. The relation kind renders as `knowledge_exposure_derived_from`.
4. The CLI keeps the existing `why --entity` surface; adopted Knowledge subjects
   now show an outgoing edge to the Exposure that supplied their source version.
5. This slice is read-only. It does not add `why --exposure`, adoption writes,
   replacement policy, external-source resolution, or federation.

## Consequences

- Adopted Knowledge can now be explained after the creation command output is no
  longer available.
- The public `why` result model can represent a non-Entity object endpoint
  without making all ObjectIdentity families query subjects.

## Implementation Findings

- None.
