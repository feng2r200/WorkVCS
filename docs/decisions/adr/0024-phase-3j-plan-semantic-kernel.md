# ADR-0024: Phase 3J Plan Semantic Kernel

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Plan/entity/versioning requirements.

## Context

Phase 3I closed dependency readiness for the read-only runnable projection.
The remaining `next` gaps include active scope/Plan path, executable Task
descendant traversal, explicit manual order, priority direction/final
ordering, final equal-candidate tie-breaker, complete `next`, and claim-next.

Confirmed requirements define Plan as a versioned Work State entity with
description, constraints, strategy, Plan-scoped Assumptions, status, ordered
children, Task-graph references, and optional completion rationale. They also
state that Plan may exist without a Goal, is not claimed or executed directly,
and that Plan completion is explicit rather than derived from descendant Task
completion.

Schema v0.1 already provides the unified Entity/EntityVersion family and no
Plan-specific table. The explicit sibling order physical representation
remains Open, and primary containment requires separate `contains` relation
semantics and acyclicity rules.

## Decision

1. Phase 3J introduces only a narrow Plan semantic kernel in `workvcs-core`.
2. A Plan is represented as an Entity whose immutable `entity_kind` is `plan`.
   Its versioned semantic state is stored in `entity_version.state_json` as
   WorkVCS canonical semantic JSON.
3. The canonical Plan state shape for this slice is:

   ```json
   {
     "assumptions": [],
     "child_order": [],
     "completion_rationale": null,
     "constraints": [],
     "description": "...",
     "status": "active",
     "strategy": "...",
     "task_refs": []
   }
   ```

4. `status` is limited to the confirmed Plan lifecycle vocabulary: `active`,
   `completed`, `abandoned`, and `superseded`. Phase 3J creates only `active`
   Plans.
5. `constraints` is a canonical JSON array of strings. Phase 3J does not
   assign ordering authority beyond preserving the caller-provided list as
   versioned state.
6. `completion_rationale` is present as `null` on creation. Lifecycle
   transitions that set or clear it remain deferred.
7. `assumptions`, `child_order`, and `task_refs` are present as empty arrays
   to keep the Plan state shape stable, but Phase 3J does not implement
   Assumption identity, explicit sibling order storage, Task graph reference
   mutation, Plan containment, Goal containment, or active Plan path
   resolution.
8. Core exposes narrow Engine-owned Plan APIs for create and readback by
   WorkStateCommit. They do not expose SQLite handles, raw SQL, or public
   generic Entity CRUD.
9. Plan create reuses the existing internal entity transition kernel and
   therefore writes one atomic ChangeSet, one ChangeOperation, one
   EntityVersion, and one WorkStateCommit with the existing Branch HEAD CAS
   behavior.
10. Public generic Entity transition rejects the reserved semantic kind
    `plan`; callers must use the Plan semantic API.
11. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3J does not add business Plan CLI commands.
12. Plan lifecycle transitions, `contains`/`references` relation APIs, primary
    containment acyclicity, active scope/Plan path, executable descendant
    traversal, full `next`, claim-next, Merge, Federation, Resource adapters,
    Bundle, Checkpoint, and migrations remain deferred.

## Consequences

- WorkVCS can create and replay the first Plan semantic object without schema
  changes or a Plan-specific physical table.
- Plan readback is explicit semantic interpretation over replayed Work State,
  not a competing projection authority.
- Later containment and active Plan path slices can build on a stable Plan
  identity/state boundary without deciding sibling ordering in this slice.

## Implementation Findings

- The confirmed baseline names ordered children, Plan-scoped Assumptions, and
  Task-graph references in Plan owned state, but does not yet fix their
  physical mutation APIs or sibling order representation. Phase 3J therefore
  keeps those fields present and empty, matching the earlier Task `child_order`
  treatment.
- The Plan semantic API can reuse the Phase 2 entity transition kernel without
  changing schema v0.1.
