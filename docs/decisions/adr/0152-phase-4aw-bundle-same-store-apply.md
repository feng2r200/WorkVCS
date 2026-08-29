# ADR-0152: Phase 4AW Bundle Same-Store Fast-Forward Apply

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0151 introduced a deterministic preflight gate for same-Store
fast-forward Bundle import. The next required tool capability is to make a
validated same-Store task-only Bundle actually advance a stale local copy
without partially moving Branch state.

## Decision

1. Phase 4AW introduces `BundleImportApplyOptions`,
   `BundleImportApplyResult`, and `Engine::apply_bundle_import`.
2. CLI adds `workvcs bundle apply-dir`.
3. Apply first runs Bundle import preflight. Only
   `same_store_fast_forward_ready` enters the write path.
4. The first apply path is intentionally scoped to same-Store task entity-only
   Bundles:
   - Entity kind must be `task`;
   - Relation versions, Knowledge federation rows, Relation membership changes,
     and checkpoint candidates remain outside this apply path.
5. Apply imports missing immutable task EntityVersion and Commit closure rows,
   validates payload digests/sizes through payload-index role+owner references,
   and treats equal existing immutable rows as no-ops.
6. Existing Branch heads are advanced with compare-and-swap after immutable rows
   have been inserted.
7. Apply records one immutable ImportAttempt and ImportAttemptOutcome in the
   same transaction as the accepted Branch fast-forward.
8. This slice does not create missing Branch refs, resolve divergence, import
   external Store Bundles, import typed Acceptance/Verification identity
   tables, or activate non-task Bundle families.

## Consequences

- A copied older Store can now import a newer same-Store task-only Bundle and
  fast-forward its existing Branch.
- More complex Bundle apply paths can be added by extending the manifest
  contract and row import coverage without changing the validated directory
  artifact boundary.

## Implementation Findings

- Current Bundle manifests export generic EntityVersion rows but do not yet
  carry enough typed identity metadata to reconstruct AcceptanceCriterion,
  VerificationRequirement, and other specialized entity tables during import.
  Phase 4AW therefore narrows apply readiness to task entity-only Bundles
  instead of silently dropping typed identity rows.
