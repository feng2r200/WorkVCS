# ADR-0026: Phase 3L Runnable Containment Scope

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Focus, containment, and `next` precedence requirements.

## Context

Phase 3G introduced read-only Runnable Task Projection for an active
Session's active Workspace and Branch. Phase 3I added dependency readiness.
Phase 3K added primary `contains` relation creation and readback for Plan/Task
endpoints.

Confirmed `next` resolution evaluates active scope/Plan path and executable
Task descendants before dependency readiness, lifecycle eligibility, priority,
manual order, and Claim coordination. The full `next` resolver, `references`,
explicit sibling order storage, priority direction, final tie-breaker, and
claim-next remain outside this slice.

## Decision

1. Phase 3L updates Runnable Task Projection to consume Session Focus and
   primary containment only for focused Plan/Task scope filtering.
2. The projection remains read-only. It creates no Event, ChangeSet,
   ChangeOperation, WorkStateCommit, Branch HEAD movement, Claim, Focus,
   Session mutation, or Task status change.
3. A Session with no Focus retains the existing workspace/branch-wide
   projection behavior and continues to report active scope/Plan path and
   executable Task descendants as deferred dimensions.
4. With a Plan Focus, the projection returns only current Task descendants
   reachable from that Plan through primary containment.
5. With a Task Focus, the projection returns the focused Task plus current
   Task descendants reachable from it through primary containment.
6. When a Focus path is present, Phase 3L validates it as a primary
   containment chain. The first path entry has no incoming relation, the last
   path entry must be the Focus entity, and every later entry must name the
   primary `contains` relation from the previous path entity to that entry.
7. Unsupported Focus entity kinds and malformed Focus paths are rejected as
   structured runtime errors rather than guessed.
8. Current primary containment graph corruption needed by focused projection,
   including duplicate current primary parents, self edges, or cycles, is
   rejected instead of silently producing candidates.
9. Dependency readiness is still computed from the full active Branch task
   graph before focused candidate filtering. An in-scope Task can therefore
   remain blocked by an out-of-scope prerequisite.
10. When a valid Plan/Task Focus scope is applied, active scope/Plan path and
    executable Task descendants are removed from the projection's deferred
    dimensions. Explicit manual order and final equal-candidate tie-breaker
    remain deferred.
11. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3L does not add business runnable or `next` commands.

## Consequences

- Runnable Task Projection now respects explicit active Focus scope when a
  Plan or Task has been selected by the Session.
- The projection can use Phase 3K structural facts without conflating
  containment with dependency, order, priority, lifecycle, or Claim ownership.
- No-Focus Sessions remain compatible with the earlier workspace-wide
  projection while honestly reporting unresolved scope dimensions.

## Implementation Findings

- Existing Focus runtime allows generic entity paths and only validates that
  referenced entities and relations exist at the active Branch head. Phase 3L
  therefore validates primary containment path semantics in the runnable
  projection instead of retroactively narrowing Focus storage.
- Because `references` and multi-Plan path selection are still deferred,
  Phase 3L only treats primary containment as an implemented active-scope path
  authority.
