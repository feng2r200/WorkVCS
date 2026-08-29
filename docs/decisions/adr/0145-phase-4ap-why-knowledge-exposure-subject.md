# ADR-0145: Phase 4AP Why KnowledgeExposure Subject

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0144 made adopted Knowledge subjects show their outgoing
`knowledge_exposure_derived_from` edge in `why`. The complementary query is to
start from a KnowledgeExposure and ask which local Knowledge currently depends
on it.

## Decision

1. Phase 4AP adds KnowledgeExposure as a `why` subject.
2. CLI extends `workvcs why` with `--exposure` as a mutually exclusive subject
   alongside `--entity` and `--evidence`.
3. Resolving a KnowledgeExposure subject validates that the Exposure exists in
   the Store. It is not required to be active, because historical Exposure
   provenance remains queryable after withdrawal.
4. The relation scan reuses the Phase 4AO KnowledgeExposure endpoint support.
   Matching edges render as incoming `knowledge_exposure_derived_from` edges.
5. This slice is read-only. It does not create adoption, restore withdrawn
   Exposures, define stale-source policy, or add cross-Store federation.

## Consequences

- A tool can now answer both sides of adoption provenance:
  why this Knowledge exists, and which Knowledge adopted this Exposure.
- `why` still remains bounded to current WorkState relations for the selected
  target commit or branch head.

## Implementation Findings

- None.
