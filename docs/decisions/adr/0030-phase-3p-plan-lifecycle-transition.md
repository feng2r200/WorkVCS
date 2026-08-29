# ADR-0030: Phase 3P Plan Lifecycle Transition

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Plan lifecycle requirements.

## Context

Phase 3J introduced the narrow Plan semantic kernel with active Plan creation
and historical `plan_at` readback. Later slices added containment and Goal
lifecycle support, but Plan lifecycle transitions remained deferred.

Confirmed requirements define Plan states as `active`, `completed`,
`abandoned`, and `superseded`. Plan completion and abandonment are explicit.
Completed or abandoned Plans may be explicitly reopened with rationale. A
superseded Plan requires supersession-aware resolution and cannot use an
ordinary reopen.

## Decision

1. Phase 3P introduces only a narrow Plan lifecycle transition API in
   `workvcs-core`.
2. The public facade remains Engine-owned and Plan-specific:
   `Engine::transition_plan`.
3. The transition target is an existing Plan Entity version at an expected
   Branch head. The caller supplies the Plan Entity id, expected current Plan
   EntityVersion id, next Plan status, and any required rationale.
4. A successful transition preserves Plan identity, description, constraints,
   strategy, empty `assumptions`, empty `child_order`, and empty `task_refs`;
   it updates only `status` and `completion_rationale`.
5. Supported ordinary lifecycle edges are:

   ```text
   active -> completed
   active -> abandoned
   completed -> active
   abandoned -> active
   ```

6. Direct `completed -> abandoned` and `abandoned -> completed` transitions
   are rejected. A terminal Plan must be explicitly reopened before entering
   the other ordinary terminal state.
7. No-op same-status transitions are rejected.
8. Ordinary transitions involving `superseded` are rejected. Supersession
   remains deferred until a supersession-aware relation and resolution contract
   is implemented.
9. Completion may carry an optional non-empty `completion_rationale` in Plan
   state. If present, the same text is used by default as canonical operation
   rationale.
10. Abandonment requires a non-empty operation rationale but does not store
    that text in `completion_rationale`.
11. Reopening from `completed` or `abandoned` requires a non-empty operation
    rationale and clears `completion_rationale` back to `null`.
12. Plan readback now validates `completion_rationale` consistency:
    non-completed Plan states must not carry `completion_rationale`, and a
    present completion rationale must be non-empty.
13. Plan lifecycle transition reuses the existing Entity transition kernel and
    therefore writes one atomic ChangeSet, one ChangeOperation, one
    EntityVersion, and one WorkStateCommit with the existing Branch HEAD
    compare-and-swap and expected EntityVersion checks.
14. Failed lifecycle transitions are structured errors and write no partial
    authoritative history rows.
15. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3P does not add business Plan CLI commands.
16. Phase 3P does not implement Plan supersession, Goal readiness hints,
    references, sibling ordering, Goal-focused runnable projection, `next`,
    claim-next, Merge, Federation, Resource adapters, Bundle, Checkpoint, or
    migrations.

## Consequences

- WorkVCS can now explicitly complete, abandon, and reopen Plans while
  preserving immutable historical snapshots.
- Completed Plan state can carry an optional stable completion rationale.
- Supersession remains closed off from ordinary reopen so later relation-aware
  implementation cannot be bypassed.

## Implementation Findings

- No schema change is required. Plan lifecycle uses the same
  Entity/EntityVersion and WorkStateCommit boundary as Plan creation.
- The confirmed Plan state field is named `completion_rationale`, not general
  terminal rationale. Phase 3P therefore stores completion rationale only for
  `completed` Plans. Abandonment rationale is stored as operation provenance
  rather than in Plan state.
