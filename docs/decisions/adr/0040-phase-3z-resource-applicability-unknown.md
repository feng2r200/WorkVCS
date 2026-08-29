# ADR-0040: Phase 3Z Resource Applicability Unknown Projection

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Verification applicability, Resource Basis, and
  non-materialized projection boundaries.

## Context

Phase 3Y made Resource Basis part of the immutable Verification defining
closure. The confirmed applicability model distinguishes historical
Verification result from current branch-sensitive applicability:

```text
any stale        -> stale
else any unknown -> unknown
else             -> applicable
```

An unavailable or incomparable Resource yields `unknown`, and `unknown` cannot
satisfy a mandatory Acceptance Criterion. Core still has no Resource Adapter or
Applicability Stamp implementation, so it cannot prove a Resource Basis remains
applicable.

## Decision

1. Phase 3Z updates the existing on-demand Acceptance Criterion effective
   projection to account for Resource Basis.
2. Work-State Basis keeps the existing rule: if any recorded semantic
   dependency is absent or at a different current EntityVersion, the
   Verification is stale.
3. If Work-State Basis is applicable but the Verification has one or more
   Resource Basis entries, Core returns internal Verification applicability
   `unknown` until a later slice provides adapter/stamp comparison evidence.
4. Existing `AcceptanceCriterionEffectiveStatus::Stale` remains the public
   effective status for historical judgments whose applicability is stale or
   unknown, matching the current V0.1 projection vocabulary.
5. Work-state-only passed Verifications remain applicable and can still satisfy
   mandatory Acceptance Criteria.
6. Phase 3Z does not write `verification_applicability_cache` or
   `applicability_resource_stamp`, does not implement Resource adapters or
   drift comparison, and does not add business CLI commands.

## Consequences

- A passed Verification with Resource Basis is no longer incorrectly treated as
  applicable merely because its Work-State dependencies still match.
- Mandatory Task completion remains conservative: Resource-backed judgments
  require later comparison evidence before they can satisfy the gate.
- Later cache/stamp work can replace the temporary internal `unknown`
  calculation without changing the public effective status vocabulary.

## Implementation Findings

- No schema change is required. This slice only changes the in-memory derived
  projection calculation.
