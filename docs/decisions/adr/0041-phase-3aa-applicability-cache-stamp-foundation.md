# ADR-0041: Phase 3AA Applicability Cache Stamp Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Verification Resource Basis / applicability boundaries.

## Context

Phase 3Y persists Resource Basis on Verification records. Phase 3Z made
Resource-backed Verification applicability conservative until resource
comparison evidence exists. The schema already contains
`verification_applicability_cache` and `applicability_resource_stamp`, but Core
did not yet expose a way to record or read that derived comparison state.

## Decision

1. Phase 3AA adds an Engine facade for recording and reading Verification
   applicability cache snapshots.
2. The cache is derived state. Recording it does not create a WorkState commit,
   Event, Changeset, EntityVersion, or RelationVersion.
3. A cache record is branch scoped. Recording requires the branch head to equal
   the evaluated Verification commit, and the branch workspace must match the
   Verification workspace.
4. Callers provide Resource Stamp observations, not the final applicability.
   Core computes `applicable`, `stale`, or `unknown` from the Verification
   Work-State Basis and Resource Basis.
5. Resource Basis comparison is complete-set only: every basis ordinal must have
   exactly one stamp. Missing or extra stamps are invalid.
6. Observed stamps compare `observed_fingerprint` with the baseline
   fingerprint. Fingerprint mismatch is `stale`; unavailable or errored
   observations are `unknown`.
7. A stamp may point to a concrete Resource Observation only when that
   observation belongs to the same resource and matches adapter kind, adapter
   schema version, and observed fingerprint.
8. Readback validates stored canonical detail JSON, row vocabulary, contiguous
   ordinals, basis/stamp fixed points, and recomputes applicability before
   returning the snapshot.
9. This slice does not implement Resource adapters, filesystem/Git/remote
   observation, drift algorithms beyond digest equality, business CLI commands,
   projection materialization, or branch-aware cache consumption by Acceptance
   Criterion effective status.

## Consequences

- Core now has a narrow persistence boundary for externally computed Resource
  applicability comparisons.
- Existing commit-only effective-status APIs remain conservative for
  Resource-backed Verification until a later branch-aware projection consumes
  the cache.
- Complete-set stamping is intentionally strict so partial Resource observation
  cannot be mistaken for an applicable Verification.

## Implementation Findings

- The existing `acceptance_criterion_effective_status(commit_id, ac_id)` API has
  no branch parameter, while applicability cache rows are branch scoped. Cache
  consumption therefore belongs in a later branch-aware projection slice rather
  than in Phase 3AA.
