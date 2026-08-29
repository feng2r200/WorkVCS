# ADR-0023: Phase 3I Runnable Dependency Readiness

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed scheduling/readiness requirements.

## Context

Phase 3G introduced read-only Runnable Task Projection with dependency
readiness deferred. Phase 3H introduced semantic Task-to-Task scheduling
relations for:

```text
depends_on      dependent -> prerequisite
ordered_before  earlier -> later
```

Confirmed requirements state that dependency blocking is a derived projection
and must not rewrite Task status. They also state that manual order cannot make
a dependency-blocked or lifecycle-ineligible Task runnable.

The full `next` resolver still depends on active scope/Plan path, executable
Task descendant traversal, priority direction/final ordering, manual order
selection, and the final equal-candidate tie-breaker. Those remain out of
scope for this slice.

## Decision

1. Phase 3I updates Runnable Task Projection to evaluate dependency readiness
   from current Phase 3H scheduling relation snapshots.
2. The projection remains read-only. It creates no Event, ChangeSet,
   ChangeOperation, WorkStateCommit, Branch HEAD movement, Claim, Focus,
   Session mutation, or Task status change.
3. Runtime code consumes scheduling relations through the history semantic read
   API. It must not independently interpret the relation tables.
4. A `depends_on` relation is interpreted as dependent -> prerequisite.
5. A candidate Task is dependency-ready only when every current prerequisite
   Task in its dependency closure has status `done`.
6. A Task with an unsatisfied prerequisite is not runnable and reports a
   dependency blocked reason.
7. Explicit Task status `blocked` remains separate from dependency blocking. A
   `blocked`, `pending`, or `in_progress` prerequisite is not satisfied for
   dependency readiness.
8. `ordered_before` remains independent from readiness and does not affect
   runnable status in Phase 3I.
9. Dependency cycles remain rejected at creation by Phase 3H. If corrupted
   history contains a current cycle, the runnable projection reports a
   structured error instead of silently producing readiness.
10. Dependency readiness is removed from the projection's deferred dimensions.
    Active scope/Plan path, executable Task descendants, explicit manual
    order, priority direction/final ordering, final equal-candidate
    tie-breaker, complete `next`, and claim-next remain deferred.
11. Existing lifecycle and Claim coordination still apply. Dependency readiness
    cannot make a lifecycle-ineligible or claim-blocked Task runnable.
12. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3I does not add business runnable or `next` commands.

## Consequences

- Runnable Task Projection now reflects explicit Task dependencies without
  mutating Task state.
- Later `next` work can build on an already semantic dependency readiness
  dimension.
- Manual order and priority remain intentionally unselected as ordering
  authorities until their unresolved implementation details are frozen.

## Implementation Findings

- The confirmed baseline does not yet define a richer prerequisite satisfaction
  policy, such as acceptance criteria effective verification, failed/cancelled
  prerequisites, supersession, or executable composite completion. Phase 3I
  therefore uses the narrow fixed rule: only `done` prerequisites satisfy
  dependency readiness.
- The final priority direction remains unfixed. Phase 3I continues to report
  priority values without using them for authoritative ordering.
