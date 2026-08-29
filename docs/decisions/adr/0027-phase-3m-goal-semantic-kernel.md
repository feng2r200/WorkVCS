# ADR-0027: Phase 3M Goal Semantic Kernel

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Goal requirements.

## Context

Confirmed WorkVCS domain requirements define Goal as a versioned Work State
entity. A Goal may be created before work or discovered after Plans and Tasks
already exist. Goal core states are `active`, `achieved`, and `abandoned`;
achievement, abandonment, and reopening are explicit and provenance-bearing.
Goal replacement is expressed through relation.

The current Rust implementation has Task, Acceptance Criterion, Verification,
Session/Claim runtime, runnable projection, Plan, and primary containment
foundation slices. Goal identity and state do not yet exist, so Goal
containment and Goal-scoped references remain deferred.

## Decision

1. Phase 3M introduces only a narrow Goal semantic kernel in `workvcs-core`.
2. Goal is persisted through the existing Entity/EntityVersion,
   ChangeSet/ChangeOperation, WorkStateCommit, Branch HEAD CAS, and replay
   path.
3. The public surface remains Engine-owned and semantic:
   `Engine::create_goal` and `Engine::goal_at`.
4. Goal state uses canonical JSON with this Phase 3M shape:

   ```json
   {
     "description": "...",
     "plan_refs": [],
     "status": "active",
     "subgoals": [],
     "terminal_rationale": null,
     "work_refs": []
   }
   ```

5. `status` is limited to the confirmed Goal lifecycle vocabulary: `active`,
   `achieved`, and `abandoned`. Phase 3M creates only `active` Goals.
6. `terminal_rationale` is present as `null` for active Goals. Transitions
   that set or clear it remain deferred.
7. `subgoals`, `plan_refs`, and `work_refs` are present but empty arrays to
   keep the Goal state shape stable. Phase 3M does not implement subgoal
   mutation, `references`, Goal containment, or late attachment of existing
   Plans/Tasks.
8. `goal_at` reads a historical Goal snapshot from replayed Work State, then
   validates entity kind, schema version, canonical fixed-point JSON, digest,
   status vocabulary, and the Phase 3M state shape.
9. Public generic Entity transition rejects the reserved semantic kind `goal`
   for create and update. Goal mutation must go through Goal semantic APIs.
10. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3M does not add business Goal CLI commands.
11. Goal achievement, abandonment, reopening, replacement/supersession,
    readiness hints, Goal containment, `references`, Plan/Task attachment,
    full `next`, claim-next, Merge, Federation, Resource adapters, Bundle,
    Checkpoint, and migrations remain deferred.

## Consequences

- WorkVCS can now persist and replay Goal identity and active Goal state
  without pretending terminal Goal workflows are implemented.
- Later Goal containment and Goal lifecycle slices can build on the same
  semantic Entity/EntityVersion boundary used by Task and Plan.
- Generic Entity transition remains a low-level escape hatch for unreserved
  exploratory entities, but cannot bypass confirmed Goal semantics.

## Implementation Findings

- Confirmed requirements allow late-discovered Goals and later attachment of
  existing Plans/Tasks, but do not require Goal creation to attach those objects
  in the same operation. Phase 3M keeps attachment out of the creation API.
- The schema has no Goal-specific companion table. The existing generic Entity
  family is sufficient for this narrow semantic kernel, matching the Plan and
  Task semantic slices.
