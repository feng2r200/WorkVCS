# ADR-0042: Phase 3AB Branch-Aware Applicability Projection

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed branch-sensitive Verification applicability model.

## Context

Phase 3AA added branch-scoped `verification_applicability_cache` and
`applicability_resource_stamp` read/write support. The existing Acceptance
Criterion effective-status API is commit-only, while Resource applicability is
confirmed as branch-sensitive derived state.

## Decision

1. Phase 3AB keeps the existing commit-only
   `acceptance_criterion_effective_status(commit_id, ac_id)` behavior unchanged.
2. Phase 3AB adds `acceptance_criterion_effective_status_for_branch(branch_id,
   ac_id)` to project status at the current branch head.
3. Branch-aware projection consumes a Verification applicability cache row only
   when it belongs to the same branch and was evaluated at the current branch
   head.
4. Missing cache, old-head cache, unavailable Resource comparison, errored
   Resource comparison, or deterministic Resource drift still project as public
   `stale` for Acceptance Criterion effective status.
5. Work-State Basis still dominates: if semantic dependencies no longer match
   the branch head WorkState, cached Resource applicability cannot make the
   Verification applicable.
6. Mandatory Task completion now uses the branch-aware projection, so a current
   applicable Resource cache can satisfy the existing semantic gate.
7. This slice does not implement Resource adapters, cache refresh automation,
   materialized projections, business CLI commands, merge semantics, or remote
   synchronization.

## Consequences

- Resource-backed Verifications can now satisfy mandatory Acceptance Criteria
  once a current branch cache proves applicability.
- Old Resource applicability cache rows do not silently authorize later branch
  heads.
- Existing commit-addressed historical queries remain stable and conservative.

## Implementation Findings

- No schema change is required. The existing branch-scoped cache schema matches
  the branch-aware projection boundary.
