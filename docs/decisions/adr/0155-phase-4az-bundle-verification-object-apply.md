# ADR-0155: Phase 4AZ Bundle Verification Object Apply Foundation

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0154 enabled same-Store Bundle apply for RelationVersion and
RelationMembershipChange rows. Verification commits can still depend on
ObjectIdentity-backed Evidence, Resource, ResourceObservation, and
Verification Basis rows that are not themselves WorkState entity or relation
membership rows. Without those closure rows, a same-Store Bundle could carry a
Verification entity and its relations but fail to replay or query the
Verification semantics after import.

## Decision

1. Bundle manifests now include explicit closure refs for ContentObject,
   Evidence, EvidenceContent, Resource, ResourceObservation,
   VerificationBasis, VerificationResourceBasis, and
   VerificationSemanticDependency rows.
2. Bundle payload indexes now carry the canonical JSON payloads needed to
   reconstruct those rows: content format metadata, evidence metadata,
   resource observation summary, verification basis, and verification resource
   scope payload JSON.
3. Same-Store Bundle apply imports the verification object-family closure
   before RelationVersion and Commit closure import, so `verifies` and
   `evidenced_by` relations can resolve their endpoints and
   `verification_at` can reconstruct semantic state.
4. Existing identical rows remain no-ops. Conflicting object identity kind,
   metadata, basis rows, content metadata, or resource observation rows reject
   the import as immutable import invalid.
5. Source-session-bound Evidence or ResourceObservation import remains outside
   this slice. The support gate keeps those Bundles non-applicable until the
   Session provenance object-family import is implemented.

## Consequences

- A stale same-Store checkout can now fast-forward through a Verification commit
  whose semantic state depends on Evidence and Resource Basis rows.
- Verification applicability cache, ResourceBinding, WorkspaceResource
  association, Session provenance import, cross-Store import, missing Branch
  creation, Knowledge federation, and Checkpoint candidate apply remain
  deferred.

## Implementation Findings

- Verification/Evidence/Resource closure rows did not require a schema change:
  all needed authoritative rows already exist in `schema-v0.1.sql`.
- Bundle payloads remain canonical JSON payloads. Raw Evidence or Resource
  content bytes are still represented by ContentObject digest and metadata
  records, not embedded as Bundle payload bytes in this slice.
