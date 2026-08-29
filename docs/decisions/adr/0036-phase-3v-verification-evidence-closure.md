# ADR-0036: Phase 3V Verification Evidence Closure

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Evidence, ContentObject, Verification closure, Relation,
  WorkState, and replay requirements.

## Context

Phase 3D introduced immutable Verification judgments with a work-state-only
Basis and a defining `verifies` Relation. It intentionally left the Evidence
set empty while preserving an `evidence` field in Verification state. Phase 3U
made current `verifies` neighborhoods visible to `why`, but the
VerificationEvidence family remains deferred because no Evidence object or
`evidenced_by` Relation exists yet.

The confirmed domain model states that Evidence is ObjectIdentity-backed
immutable provenance, not a content blob and not an Entity. A Verification or
another semantic object may be `evidenced_by` Evidence. ContentObject identity
is the raw-byte digest and is separate from storage backend/location.

## Decision

1. Phase 3V introduces only the immutable Evidence and Verification evidence
   closure slice in `workvcs-core`.
2. A new strongly typed `EvidenceId` follows the existing UUIDv7 identity
   pattern.
3. Evidence creation is exposed only through the Engine facade:

   ```text
   Engine::create_evidence(...)
   Engine::evidence(...)
   ```

4. Evidence is written to `object_identity` and `evidence`. It is not written
   to `entity`, has no EntityVersion, and is not a WorkState member.
5. Evidence metadata is a WorkVCS canonical JSON object. Evidence may be
   metadata-only.
6. Evidence may reference zero or more ContentObjects through
   `evidence_content`.
7. ContentObject identity is the V1 raw-byte BLAKE3-256 digest. Phase 3V writes
   only digest metadata (`content_object`) and Evidence links
   (`evidence_content`). It does not store raw bytes or write
   `content_storage_location`.
8. Existing ContentObject rows can be reused only when digest, size, media
   type, and canonical format metadata agree.
9. Existing Evidence can be reused by multiple Verifications.
10. `VerificationCreateOptions` accepts a duplicate-free Evidence set.
11. Verification state records that Evidence set in deterministic EvidenceId
    order.
12. The same Verification creation commit writes one immutable `evidenced_by`
    Relation for each Evidence item:

    ```text
    verification -> evidence
    ```

13. The resulting WorkState atomically includes the Verification EntityVersion,
    the defining `verifies` RelationVersion, and every defining
    `evidenced_by` RelationVersion.
14. `verification_at` validates that the stored Verification evidence set
    matches the current `evidenced_by` Relations and that every target object is
    Evidence.
15. Creating a Verification without Evidence remains valid and preserves the
    existing empty Evidence behavior.
16. Phase 3V does not implement Evidence blob storage, storage-location writes,
    Resource, ResourceObservation, Verification Resource Basis,
    Artifact/Input Basis, adapter-backed drift comparison, Evidence retention
    policy, `why` Evidence endpoint rendering, context, `next`, restore,
    merge, projection materialization, migrations, or business CLI commands.

## Consequences

- Verification now has an immutable defining Evidence set and canonical
  `evidenced_by` edges without introducing duplicate verification-evidence
  tables.
- Evidence can be captured once and reused across multiple Verification
  judgments.
- Later Resource and Artifact/Input Basis slices can reference the same
  Evidence and ContentObject foundations.
- Later `why` work still needs a public result model that can represent
  ObjectIdentity endpoints, because the current `why` result uses EntityId
  fields and cannot directly render Evidence targets.

## Implementation Findings

- The physical schema already supports Evidence, ContentObject, and
  `evidenced_by` through ObjectIdentity-backed Relations. No schema change is
  required for this slice.
- The current `why` result model is Entity-focused. Rendering
  `verification -> evidence` edges would require a deliberate public query
  shape change. Phase 3V therefore records and validates the closure but leaves
  `why` Evidence endpoint rendering deferred.
- Introducing `evidenced_by` exposed an implementation assumption in the
  verifies relation scanner: it joined endpoint Entity rows before deciding
  whether the relation was actually `verifies`. Since Evidence is not an
  Entity, the scanner now identifies relation type first and only then requires
  Verification/target Entity endpoint kinds for true `verifies` rows.
