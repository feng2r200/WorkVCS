# ADR-0037: Phase 3W Why Evidence Neighborhood

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed `why`, Evidence, Verification closure, Relation,
  WorkState, and replay requirements.

## Context

Phase 3T introduced a read-only structural `why` query over containment and
structural references. Phase 3U added current `verifies` relation
neighborhoods. Phase 3V added immutable Evidence objects, ContentObject
metadata, Verification evidence sets, and current `evidenced_by` Relations,
but deliberately left Evidence endpoint rendering out of `why` because the
public result shape was Entity-only.

The confirmed relationship model says `evidenced_by` attaches immutable
Evidence to a semantic object. Evidence is ObjectIdentity-backed provenance,
not an Entity and not a WorkState member. The selected current WorkState
contains the `evidenced_by` RelationVersion, not the Evidence object itself.

## Decision

1. Phase 3W introduces only the Evidence-aware `why` neighborhood slice in
   `workvcs-core`.
2. `why` remains exposed only through the Engine facade:

   ```text
   Engine::why(...)
   ```

3. The existing Entity-subject constructor remains valid:

   ```text
   WhyQueryOptions::new(target, entity_id)
   ```

4. An Evidence-subject constructor is added:

   ```text
   WhyQueryOptions::for_evidence(target, evidence_id)
   ```

5. `WhyRelationEdge` uses typed source and target endpoints instead of
   Entity-only endpoint fields. Endpoints may be:

   ```text
   Entity { entity_kind, entity_id }
   Evidence { evidence_id }
   ```

6. `WhyRelationKind` includes `EvidencedBy`.
7. For an Entity subject, `why` still requires the subject Entity to be current
   in the selected WorkState and returns its current EntityVersion ID.
8. For an Evidence subject, `why` requires the Evidence object to exist, then
   renders current incoming `evidenced_by` relations selected by the target
   WorkState. Evidence itself is not treated as a WorkState member.
9. `why` includes outgoing `evidenced_by` edges for current Verification
   subjects and incoming `evidenced_by` edges for Evidence subjects.
10. Current `evidenced_by` rendering validates that the source Verification is
    present in the selected WorkState and that the relation target is an
    Evidence object.
11. The `VerificationEvidence` deferred relation-family marker is removed,
    because this slice renders that family. Evolution and Epistemic relation
    families remain deferred.
12. Phase 3W does not implement Resource, ResourceObservation, Verification
    Resource Basis, Artifact/Input Basis, adapter-backed drift comparison,
    Evidence blob storage, Evidence retention policy, context, `next`, restore,
    merge, projection materialization, migrations, or business CLI commands.

## Consequences

- `why` can now explain the defining Evidence neighborhood of a Verification
  without duplicating Evidence state or treating Evidence as an Entity.
- Callers can ask why an Evidence object matters in the current WorkState by
  querying incoming current `evidenced_by` relations.
- Existing containment, structural reference, and `verifies` explanations keep
  the same semantics, but callers must read endpoints from the new typed
  endpoint fields.
- Later Resource and applicability slices can attach richer Evidence and
  Verification Basis semantics without changing the Relation authority model.

## Implementation Findings

- No schema change is required. The current `relation`,
  `relation_version`, `object_identity`, `evidence`, and WorkState replay
  paths already carry the necessary authority.
- Evidence-subject `why` is intentionally relation-scoped: it reports current
  relation usage in the selected WorkState, while Evidence existence remains
  Store-level immutable provenance.
