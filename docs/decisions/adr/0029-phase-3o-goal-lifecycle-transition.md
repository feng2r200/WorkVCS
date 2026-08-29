# ADR-0029: Phase 3O Goal Lifecycle Transition

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Goal lifecycle requirements.

## Context

Phase 3M introduced the narrow Goal semantic kernel with active Goal creation
and historical `goal_at` readback. Phase 3N allowed Goal to participate in
primary containment relations. Confirmed requirements define Goal states as
`active`, `achieved`, and `abandoned`; Goal achievement, abandonment, and
reopening are explicit and carry required provenance. Descendant completion
may produce a readiness hint later, but it never automatically achieves a
Goal.

## Decision

1. Phase 3O introduces only a narrow Goal lifecycle transition API in
   `workvcs-core`.
2. The public facade remains Engine-owned and Goal-specific:
   `Engine::transition_goal`.
3. The transition target is an existing Goal Entity version at an expected
   Branch head. The caller supplies the Goal Entity id, expected current Goal
   EntityVersion id, next Goal status, and a rationale string.
4. A successful transition preserves Goal identity, description, empty
   `subgoals`, empty `plan_refs`, and empty `work_refs`; it updates only
   `status` and `terminal_rationale`.
5. Supported lifecycle edges are:

   ```text
   active -> achieved
   active -> abandoned
   achieved -> active
   abandoned -> active
   ```

6. Direct `achieved -> abandoned` and `abandoned -> achieved` transitions are
   rejected. A terminal Goal must be explicitly reopened before entering the
   other terminal state.
7. No-op same-status transitions are rejected.
8. Entering `achieved` or `abandoned` requires a non-empty terminal rationale,
   stored in Goal state as `terminal_rationale`.
9. Reopening from `achieved` or `abandoned` requires a non-empty operation
   rationale and clears Goal state `terminal_rationale` back to `null`.
10. The same rationale string is stored by default as a canonical operation
    rationale object with a `reason` field. If a caller overrides the operation
    rationale, it must still be a non-empty canonical object.
11. Goal readback now validates status and `terminal_rationale` consistency:
    `active` requires `terminal_rationale = null`; terminal statuses require a
    non-empty `terminal_rationale`.
12. Goal lifecycle transition reuses the existing Entity transition kernel and
    therefore writes one atomic ChangeSet, one ChangeOperation, one
    EntityVersion, and one WorkStateCommit with the existing Branch HEAD
    compare-and-swap and expected EntityVersion checks.
13. Failed lifecycle transitions are structured errors and write no partial
    authoritative history rows.
14. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3O does not add business Goal CLI commands.
15. Phase 3O does not implement Plan lifecycle transitions, Goal replacement,
    references, readiness hints, Goal-focused runnable projection, `next`,
    claim-next, Merge, Federation, Resource adapters, Bundle, Checkpoint, or
    migrations.

## Consequences

- WorkVCS can now explicitly achieve, abandon, and reopen Goals while
  preserving immutable historical snapshots.
- Goal terminal rationale becomes part of the validated semantic state rather
  than an unchecked placeholder.
- Future readiness projection can suggest Goal completion without changing
  the invariant that achievement remains an explicit semantic operation.

## Implementation Findings

- No schema change is required. Goal lifecycle uses the same
  Entity/EntityVersion and WorkStateCommit boundary as Goal creation.
- Phase 3M intentionally allowed terminal status vocabulary before terminal
  transitions existed. Phase 3O tightens readback so terminal Goal states are
  only accepted when their required `terminal_rationale` is present and
  non-empty.
