# ADR-0141: Phase 4AL KnowledgeExposure Adoption Candidate

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

The confirmed federation model distinguishes read-only consulting from explicit
adoption. Adoption must create Workspace-local Knowledge, preserve
Exposure/source-version provenance, and eventually add a `derived_from` edge to
the KnowledgeExposure. Existing slices have created KnowledgeSpaces,
KnowledgeExposures, source-status refresh, and the main KnowledgeSpace Exposure
read sets. Before mutating a target Workspace, tools need a read-only way to
resolve the exact source KnowledgeVersion and the provenance payload that a
later adoption commit would carry.

## Decision

1. Phase 4AL adds `knowledge_exposure_adoption_candidate` to the Engine facade.
2. The candidate is read-only and accepts one `ExposureId`.
3. The candidate is available only for `active` Exposures. Withdrawn Exposures
   remain historical and are rejected as adoption candidates.
4. The candidate loads the Exposure's immutable local source KnowledgeVersion,
   verifies that its digest matches the Exposure binding, and returns the source
   Knowledge state.
5. The result includes canonical adoption provenance containing the source
   Store, Workspace, Knowledge identity, KnowledgeVersion, Knowledge state
   digest, KnowledgeSpace, Exposure id, and current source-status projection.
6. CLI adds `store knowledge-exposure-adoption-candidate` and renders the source
   state plus `adoption_provenance_json`.
7. This slice does not create Workspace-local Knowledge, create the
   `derived_from -> KnowledgeExposure` relation, choose a stale-source adoption
   policy, resolve external sources, or implement cross-Store federation.

## Consequences

- The next adoption slice can reuse a tested candidate contract instead of
  rediscovering source state and provenance rules while mutating a Workspace.
- The implementation preserves the confirmed distinction between consulting and
  adoption: this command observes the source binding but performs no Work-State
  mutation.

## Implementation Findings

- None.
