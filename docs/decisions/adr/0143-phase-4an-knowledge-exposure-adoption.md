# ADR-0143: Phase 4AN Knowledge Exposure Adoption

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0141 exposed a read-only KnowledgeExposure adoption candidate, and
ADR-0142 added the replayable `derived_from` relation from a Workspace-local
Knowledge entity to a KnowledgeExposure. The next required slice is the narrow
mutation that turns an active/current Exposure into local Knowledge while
preserving provenance.

## Decision

1. Phase 4AN adds a narrow Engine facade for adopting an active KnowledgeExposure
   into the caller's target Workspace branch.
2. Adoption is implemented as two ordinary replay-supported commits:
   first a Knowledge entity create, then a `derived_from` relation link to the
   source Exposure.
3. The adopted Knowledge copies the source Knowledge statement and scope.
4. The adopted Knowledge provenance is a canonical object containing:
   `kind = knowledge_exposure_adoption_v1`, the candidate adoption provenance,
   and the source Knowledge provenance.
5. Adoption requires the Exposure lifecycle to be active and the current source
   status to be `current`.
6. CLI adds `store knowledge-exposure-adopt`.
7. This slice does not define stale-source adoption policy, replacement,
   conflict merge, external-source resolution, cross-Store federation, or a new
   replay operation type.

## Consequences

- Adoption is immediately replayable because it reuses entity transition and
  relation membership semantics already in the storage engine.
- The final branch head is the relation commit; the intermediate Knowledge
  commit is returned explicitly for audit and follow-up operations.
- A future slice may introduce atomic grouped adoption only if the confirmed
  model requires single-commit adoption semantics.

## Implementation Findings

- None.
