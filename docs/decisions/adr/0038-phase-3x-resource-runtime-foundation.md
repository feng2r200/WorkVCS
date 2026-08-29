# ADR-0038: Phase 3X Resource Runtime Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Resource, ResourceBinding, ResourceObservation,
  ContentObject, WorkState, and Verification Basis boundaries.

## Context

Phase 3V added Evidence and ContentObject metadata. Phase 3W made Evidence
usable in `why` neighborhoods through typed relation endpoints. The next
confirmed Verification/applicability gap is Resource-backed basis and drift,
but the domain model separates Resource identity, environment binding,
Workspace association, immutable ResourceObservation, VerificationBasis, and
derived applicability.

The narrow prerequisite is therefore Resource foundation only. It must not
turn Resource changes into Branch WorkState changes and must not implement
adapter logic inside Core.

## Decision

1. Phase 3X introduces only the minimal Resource foundation in
   `workvcs-core`.
2. Resource operations are exposed only through the Engine facade:

   ```text
   Engine::create_resource(...)
   Engine::resource(...)
   Engine::bind_resource(...)
   Engine::associate_workspace_resource(...)
   Engine::record_resource_observation(...)
   Engine::resource_observation(...)
   ```

3. Resource and ResourceObservation receive strongly typed UUIDv7 IDs:
   `ResourceId` and `ResourceObservationId`.
4. A Resource is ObjectIdentity-backed immutable logical identity with an
   immutable `resource_kind`. It is not an Entity and is not selected by
   WorkState.
5. ResourceBinding is current environment-local binding/configuration. It can
   be rebound for the same Resource without creating a WorkStateCommit.
6. WorkspaceResource association is Workspace-level configuration. Association
   metadata can be updated without creating a WorkStateCommit.
7. ResourceObservation is ObjectIdentity-backed immutable provenance for one
   Resource at one time. It records Resource, adapter kind, positive adapter
   schema version, captured time, BLAKE3-256 fingerprint, canonical summary
   JSON, optional detail ContentObject digest metadata, and optional source
   Session.
8. Optional ResourceObservation detail content writes or reuses only
   ContentObject digest metadata. It does not store raw bytes and does not
   write `content_storage_location`.
9. Engine readback validates object kinds, canonical JSON fixed-point storage,
   positive schema versions, digest widths, and referenced Resource,
   Workspace, and Session existence.
10. Phase 3X does not implement Resource adapters, Resource scope semantics,
    Verification Resource Basis, Artifact/Input Basis, applicability cache or
    stamps, drift comparison, Evidence retention policy, context, `next`,
    restore, merge, projection materialization, migrations, or business CLI
    commands.

## Consequences

- Later Verification Resource Basis work can reference stable Resource IDs and
  immutable ResourceObservations.
- Later adapter work can supply normalized fingerprints without changing the
  Resource storage boundary.
- Resource rebinding and Workspace association changes remain infrastructure
  configuration, not Branch semantic state.
- Because Core still has no adapter implementation, it records caller-supplied
  normalized fingerprints and summaries rather than observing files, Git, or
  remote systems directly.

## Implementation Findings

- The existing v0.1 schema already contains `resource`, `resource_binding`,
  `workspace_resource`, `resource_observation`, `content_object`, and
  `content_storage_location`. No schema change is required for this slice.
- ResourceObservation detail content can share the same ContentObject metadata
  rules as Evidence while leaving blob storage and storage-location tracking
  deferred.
