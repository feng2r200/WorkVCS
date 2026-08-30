# ADR-0175: Phase 4BT Next Stable Tie-Breaker

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0021 intentionally reported the final equal-candidate stable tie-breaker for
`next` as Open. Later slices added `claim_next_task` and `next_work`, which use
the deterministic runnable projection order for selection.

The implementation already orders otherwise equal candidates by stable EntityId
bytes. The remaining gap is to make that behavior an explicit V0.1 rule and stop
reporting the tie-breaker as deferred.

## Decision

1. The final V0.1 stable tie-breaker for otherwise equal `next` candidates is
   ascending Task EntityId byte order.
2. This rule applies after all implemented scheduling filters and ordering
   dimensions: runnable candidates first, then Task EntityId bytes among equal
   candidates.
3. Runnable projection no longer reports
   `FinalEqualCandidateTieBreaker` in `deferred_dimensions`.
4. `claim_next_task` and `next_work` continue to select the first unclaimed
   runnable candidate from the projection, now relying on the explicit EntityId
   tie-breaker.

## Non-Goals

- This slice does not decide Task integer priority direction.
- This slice does not add explicit manual ordering.
- This slice does not change active scope, Plan path, executable descendant, or
  Claim takeover semantics.
- This slice does not change UUID generation, Entity identity, or WorkState
  digest rules.

## Consequences

- Equal-candidate `next` selection is deterministic and no longer reported as a
  missing implementation dimension.
- Existing projection ordering remains compatible with prior behavior while
  becoming an explicit tool contract.
- Manual order and priority policy can still be added later before this final
  tie-breaker without changing the tie-breaker itself.

## Implementation Findings

- No frozen-contract contradiction was found.
- The prior EntityId sort was already present as deterministic display order;
  this slice formalizes it as the V0.1 equal-candidate `next` tie-breaker.
