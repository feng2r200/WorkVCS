# ADR-0031: Phase 3Q Goal-Focused Runnable Projection

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Goal, Focus, primary containment, and runnable
  projection requirements.

## Context

Phase 3L introduced read-only Runnable Task Projection scoped by Session Focus
for Plan and Task focus entities. Phase 3N then expanded primary containment
endpoints to include Goal, but explicitly deferred Goal-focused runnable
projection. Phase 3M and Phase 3O added Goal identity and lifecycle state.

Confirmed domain requirements allow a Goal to contain Goals, Plans, and Tasks,
and allow existing Plans and Tasks to be attached under late-discovered Goals
without changing their identity. The runnable projection therefore needs to
interpret an active Goal Focus as the root of a primary containment scope while
continuing to emit Task candidates only.

## Decision

1. Phase 3Q extends Runnable Task Projection to accept an active Session Focus
   whose focus entity is a current Goal at the active Branch head.
2. The projection remains read-only. It creates no Event, ChangeSet,
   ChangeOperation, WorkStateCommit, Branch HEAD movement, Claim, Focus,
   Session mutation, Task status change, Goal status change, or projection
   cache row.
3. With a Goal Focus, the projection returns current Task descendants reachable
   from that Goal through current primary containment. The Goal itself is not a
   runnable candidate.
4. Goal descendants may traverse mixed Goal, Plan, and Task containment
   endpoints already accepted by Phase 3N. Only Task descendants are emitted as
   candidates.
5. A Focus path rooted at a Goal is validated by the same primary containment
   path rules from Phase 3L. The first path entry has no incoming relation,
   the last path entry must be the Focus entity, and every later entry must
   name the primary `contains` relation from the previous path entity to that
   entry.
6. Unsupported Focus entity kinds and malformed Goal-rooted Focus paths are
   rejected as structured runtime errors rather than guessed.
7. Current primary containment graph corruption needed by focused projection,
   including duplicate current primary parents, self edges, or cycles, remains
   rejected instead of silently producing candidates.
8. Dependency readiness is still computed from the full active Branch task
   graph before Goal-focused candidate filtering. An in-scope Task can
   therefore remain blocked by an out-of-scope prerequisite.
9. When a valid Goal Focus scope is applied, active scope/Plan path and
   executable Task descendants are removed from the projection's deferred
   dimensions, preserving the Phase 3L focused projection convention.
10. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3Q does not add business runnable, Goal, or `next` commands.
11. Phase 3Q does not implement Goal readiness hints, Goal/Plan supersession,
    `references`, sibling ordering, priority resolution, full `next`,
    claim-next, Merge, Federation, Resource adapters, Bundle, Checkpoint,
    migrations, or business CLI commands.

## Consequences

- Active work can now be viewed through an explicit Goal Focus without
  flattening Goal structure into Plan identity or Task identity.
- The existing runnable projection remains Task-candidate-only, which keeps
  Goal progress, readiness hints, and higher-level `next` semantics deferred.
- No-Focus, Plan-Focus, and Task-Focus projection behavior remain compatible
  with earlier Phase 3G, 3I, and 3L slices.

## Implementation Findings

- The existing descendant traversal already handled non-Task focus roots once
  the focus entity kind could be resolved. The implementation therefore only
  needs to add Goal to focus-kind recognition and extend conformance tests.
- The focused deferred dimension name `ActiveScopePlanPath` is preserved for
  compatibility even though Goal-rooted paths are now accepted. A later full
  `next` slice may rename or refine this public enum only with an explicit
  compatibility decision.
