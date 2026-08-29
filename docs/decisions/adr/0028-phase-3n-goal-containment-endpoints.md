# ADR-0028: Phase 3N Goal Containment Endpoints

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Goal, Plan, Task, and primary containment requirements.

## Context

Phase 3K introduced primary `contains` Relation creation and readback for
Plan/Task endpoints. Phase 3L used those relations for focused runnable
projection when the focus is a Plan or Task. Phase 3M then introduced the
narrow Goal semantic kernel on the Entity/EntityVersion history path.

Confirmed domain requirements state that a Goal may contain or reference
Plans, existing Plans and Tasks may be attached later without changing their
identity, and Plan may be contained by a Goal or another Plan. Primary
containment remains tree/forest-like in a Work State: each Entity has at most
one primary containment parent, containment must remain acyclic, and all
endpoints must belong to the same Workspace.

## Decision

1. Phase 3N expands primary containment endpoints to include Goal.
2. The same Relation storage shape, operation type
   `primary_containment.create`, RelationVersion empty canonical state,
   Branch HEAD CAS, WorkState replay, and historical readback path from Phase
   3K remain authoritative.
3. Supported primary containment pairs are:

   ```text
   Goal -> Goal
   Goal -> Plan
   Goal -> Task
   Plan -> Plan
   Plan -> Task
   Task -> Task
   ```

4. `Plan -> Goal`, `Task -> Goal`, and `Task -> Plan` are rejected as
   unsupported primary containment directions.
5. Goal endpoints are validated through `goal_at`, so containment can only
   reference Goals that are present and valid at the expected Work State.
6. The single-primary-parent, duplicate-edge, self-edge, same-Workspace, and
   acyclic graph checks continue to apply across mixed Goal/Plan/Task
   containment graphs.
7. `primary_containment_relations_at` returns Goal endpoint kinds for
   historical snapshots and validates stored Goal endpoints on replay.
8. Phase 3N does not add Goal achievement, abandonment, reopening,
   Goal readiness hints, `references`, sibling ordering, Goal-focused runnable
   projection, `next`, claim-next, Merge, Federation, Resource adapters,
   Bundle, Checkpoint, migrations, or business CLI commands.

## Consequences

- Existing Plans and Tasks can now be attached under a Goal without changing
  their identity, matching the confirmed late-discovery Goal model.
- Goal substructure can be represented as primary containment where a single
  tree/forest parent is intended, while multi-Plan reuse remains deferred to
  `references`.
- Runnable projection remains limited to the Phase 3L supported Plan/Task
  Focus semantics until a later slice explicitly defines Goal Focus behavior.

## Implementation Findings

- No schema change is required. The existing canonical Relation family already
  stores endpoint ids, relation type, discriminator, relation version state,
  membership changes, and Work State membership for this slice.
- Goal's Phase 3M state still contains empty `subgoals`, `plan_refs`, and
  `work_refs` arrays. Phase 3N records containment as Relation history rather
  than mutating those placeholder arrays, preserving the narrow Goal kernel
  boundary.
