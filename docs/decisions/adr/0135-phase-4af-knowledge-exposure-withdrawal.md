# ADR-0135: Phase 4AF KnowledgeExposure Withdrawal

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0004, ADR-0006, and INV-059 require KnowledgeExposure availability changes
to preserve append-only history. Phase 4AE created active local-source
Exposures and the current projection but did not implement lifecycle changes.

## Decision

1. Phase 4AF adds `withdraw_knowledge_exposure` to the Engine facade.
2. Withdrawal requires the caller to provide the current Exposure transition id.
   This is the optimistic guard for the linear current head.
3. Withdrawal appends one `knowledge_exposure_transition` row with lifecycle
   status `withdrawn`, `previous_transition_id` pointing at the prior current
   transition, and canonical object detail JSON.
4. Withdrawal updates `knowledge_exposure_current` to the new transition and
   status. It does not delete the Exposure, its source binding, source-status
   row, or prior transition history.
5. Re-withdrawal and stale transition ids are rejected.
6. CLI adds `store knowledge-exposure-withdraw` plus active/withdrawn list
   verification through existing list filters.
7. This slice does not implement replacement semantics, reactivation,
   source-stale refresh, external sources, adoption, KnowledgeSpace ranking, or
   Bundle transport for Exposure rows.

## Consequences

- A KnowledgeSpace can now remove an Exposure from its current available set
  while retaining the source-version binding and history for audit.
- Later replacement can be modeled as creating a new Exposure and explicitly
  withdrawing the old Exposure.

## Implementation Findings

- None.
