# ADR-0176: Phase 4BU Runnable Manual Order

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0022 introduced `ordered_before` Task scheduling relations as preferred
sibling order that does not imply dependency. ADR-0023 kept `ordered_before`
independent from readiness, and ADR-0175 made EntityId byte order the final
equal-candidate tie-breaker for `next`.

The remaining tool gap is that runnable projection and `claim_next_task` still
reported explicit manual order as deferred even though the semantic relation
facts are already available at the active Branch head.

## Decision

1. Runnable projection consumes current `ordered_before` Task scheduling
   relations from the same commit snapshot already used for dependency
   readiness.
2. `ordered_before` is interpreted as a deterministic partial order among
   candidate Tasks: source Task before target Task.
3. Manual order is applied after the current runnable/non-runnable grouping and
   before the final EntityId tie-breaker.
4. Unrelated equal candidates are ordered by EntityId bytes.
5. A cycle in the current `ordered_before` candidate graph is rejected as a
   Relation error instead of being silently ignored.
6. Runnable projection no longer reports `ExplicitManualOrder` in
   `deferred_dimensions`.
7. `claim_next_task` and `next_work` continue to select the first unclaimed
   runnable candidate from the projection, now respecting explicit manual order.

## Non-Goals

- This slice does not decide Task integer priority direction.
- This slice does not make `ordered_before` affect dependency readiness.
- This slice does not add relation update, removal, or reorder commands.
- This slice does not change active scope, Plan path, executable descendant, or
  Claim takeover semantics.

## Consequences

- Users can influence `next` selection using the already confirmed
  `ordered_before` relation without adding a second scheduling authority.
- Equal candidates still have a deterministic final fallback through ADR-0175.
- Remaining runnable projection deferred dimensions are narrowed to active
  scope/Plan path and executable descendant traversal for workspace-wide
  projections.

## Implementation Findings

- No frozen-contract contradiction was found.
- Current `ordered_before` creation does not enforce acyclicity, so projection
  validates the current candidate graph before ranking.
