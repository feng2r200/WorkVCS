# ADR-0179: Phase 4BX Shared Claim Next

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0178 added explicit shared Claim creation while keeping default
`claim_next_task` behavior exclusive. That left a small tool gap: an Agent could
join a known shared Task through `claim task --mode shared`, but it could not ask
the Engine to select the next compatible shared candidate atomically.

## Decision

1. `ClaimNextOptions::new` continues to default to `ClaimMode::Exclusive`.
2. Callers may explicitly choose shared selection through
   `ClaimNextOptions::with_mode(ClaimMode::Shared)`.
3. Exclusive `claim_next_task` preserves the previous behavior: it selects only
   candidates that are runnable and unclaimed.
4. Shared `claim_next_task` selects the first projection-ordered candidate that
   is either:
   - runnable and unclaimed; or
   - blocked only by an existing shared Claim set that does not already include
     the requesting Session.
5. The claim write, Session Focus update, Session activity update, Branch-head
   revalidation, and provenance event behavior remain unchanged.
6. CLI exposes the mode through `claim next --mode shared`; omitted mode remains
   exclusive.

## Non-Goals

- This slice does not implement Claim mode changes.
- This slice does not implement stale Claim takeover, transfer, force, or
  Session recovery.
- This slice does not make shared Claim creation automatic.
- This slice does not change `next_work`, which continues to use the default
  exclusive `claim_next_task` path.

## Consequences

- Agents can atomically join shared work from the ordered runnable projection
  without first reading and manually copying a Task id.
- Existing `claim next` callers remain command-compatible and continue to claim
  only unclaimed Tasks by default.

## Implementation Findings

- No schema change was required. The existing `claim.mode` vocabulary and
  active Claim-set validation from ADR-0178 were sufficient.
