# ADR-0025: Phase 3K Primary Containment Relations

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed structural relation requirements.

## Context

Phase 3J introduced Plan identity and state on the existing Entity/EntityVersion
history path. Confirmed requirements define `contains` as the canonical
container -> child structural relation, state that Plan and Task children may
be mixed within an ordered container, and require primary containment to be
tree/forest-like in one Work State: an object has at most one primary
containment parent and adding a primary containment edge cannot create a
cycle.

The explicit sibling order storage representation remains Open. Goal identity,
`references`, active scope/Plan path resolution, executable descendant
traversal, full `next`, and claim-next are still deferred.

## Decision

1. Phase 3K introduces only primary `contains` relation creation and readback
   for semantic endpoint kinds already implemented in code.
2. A primary containment edge is represented as a Relation with:

   ```text
   relation_type = "contains"
   relation_discriminator = "primary"
   source_object_id = container entity
   target_object_id = child entity
   ```

3. Relation state is the canonical empty object. The ChangeSet and
   ChangeOperation payload use the existing relation transition payload shape.
4. The new operation type is `primary_containment.create`, schema version 1,
   and replay treats it as a normal relation creation operation.
5. Phase 3K supports Plan -> Plan, Plan -> Task, and Task -> Task. Task ->
   Plan, Goal containment, `references`, and other entity families remain
   deferred.
6. Primary containment creation validates that both endpoints are present in
   the expected Work State, are semantic Plan/Task endpoints, belong to the
   same Workspace as the Branch head, do not form a self edge, and use a
   supported endpoint kind pair.
7. A child may have at most one current primary containment parent in the
   replayed Work State. Multi-Plan reuse remains deferred to `references`.
8. Adding a primary containment edge is rejected when it would create a current
   containment cycle.
9. Readback returns only current primary `contains` relations from replayed
   Work State. Historical relation rows do not independently drive the result.
10. Primary containment remains independent from `depends_on`, `ordered_before`,
    priority, lifecycle, verification, Session, Claim, and Plan completion.
11. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3K does not add business containment CLI commands.
12. Explicit sibling order mutation/storage, active Plan path, executable Task
    descendant traversal, complete `next`, claim-next, Merge, Federation,
    Resource adapters, Bundle, Checkpoint, and migrations remain deferred.

## Consequences

- WorkVCS can now persist and replay the structural parent/child facts needed
  by later active Plan path and executable descendant slices.
- Primary containment invariants are enforced at the semantic API boundary
  without changing schema v0.1.
- Plan/Task hierarchy facts remain separate from scheduling, priority, and
  runtime claim coordination.

## Implementation Findings

- The confirmed baseline names primary containment, but schema v0.1 does not
  define a dedicated containment table. The generic Relation family plus
  `relation_discriminator = "primary"` is sufficient for this slice and keeps
  room for future non-primary structural relation forms.
- Because Goal identity is not implemented yet, Goal containment remains
  deferred rather than represented with placeholder entity kinds.
