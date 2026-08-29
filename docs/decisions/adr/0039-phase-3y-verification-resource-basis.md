# ADR-0039: Phase 3Y Verification Resource Basis

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Verification Basis, Resource, ResourceObservation,
  WorkState, and physical schema v0.1 boundaries.

## Context

Phase 3D introduced Verification with a work-state-only basis. Phase 3V added
Evidence as an immutable defining relation set. Phase 3X added stable Resource
and immutable ResourceObservation runtime foundations. The confirmed
Verification Basis model requires Resource Basis plus Work-State Basis, and the
v0.1 schema already contains `verification_resource_basis`.

This slice connects Resource Basis to Verification creation and readback without
starting Resource adapters, drift comparison, applicability cache or stamps,
Artifact/Input Basis, or projection materialization.

## Decision

1. Phase 3Y introduces `VerificationResourceBasis` in `workvcs-core`.
2. `VerificationCreateOptions::with_resource_basis(...)` records an ordered
   Resource Basis list. Existing creation with no Resource Basis remains valid.
3. Each Resource Basis entry records:
   - Resource identity;
   - Adapter kind and positive Adapter schema-contract version;
   - Scope kind, positive scope schema-contract version, and canonical object
     `scope_payload`;
   - Optional baseline ResourceObservation;
   - Required baseline fingerprint.
4. Resource Basis order is preserved as the schema `ordinal`; it is not sorted
   as a set.
5. Verification creation validates that every referenced Resource exists. When
   a baseline ResourceObservation is supplied, it must belong to the same
   Resource and match the basis Adapter kind, Adapter schema version, and
   baseline fingerprint.
6. Verification creation writes Resource Basis into the immutable Verification
   state, `verification_basis.basis_json`, and `verification_resource_basis` in
   the same transaction.
7. Verification readback validates canonical JSON fixed points and checks that
   state JSON, `verification_basis.basis_json`, and structured
   `verification_resource_basis` rows describe the same Basis.
8. New Verification rows always include `resource_basis` in the basis object,
   using an empty array when no Resource Basis is present. Readback accepts the
   older Phase 3D two-field basis object as equivalent to empty Resource Basis.
9. Phase 3Y does not implement Resource adapters, Resource drift comparison,
   applicability cache or stamps, Artifact/Input Basis, Evidence retention
   policy, context, `next`, restore, merge, projection materialization,
   migrations, or business CLI commands.

## Consequences

- Verification can now persist Resource-backed provenance without interpreting
  Git, files, directories, or remote services in Core.
- Later applicability work can compare current Resource observations against
  the structured Resource Basis rows.
- Re-verification remains the only way to change a Verification defining
  closure.

## Implementation Findings

- The existing v0.1 schema already contained `verification_resource_basis`, so
  no schema change was required.
- Adding Resource Basis expands the Verification basis JSON shape while the
  state schema version remains `1`. Because WorkVCS is still pre-release V0.1
  implementation work, new rows write the expanded shape and readback keeps a
  narrow compatibility path for older two-field basis JSON with empty Resource
  Basis.
