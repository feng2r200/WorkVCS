# ADR-0048: Phase 3AH Claim Next

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Runtime Coordination / runnable projection boundary.

## Context

The CLI could list runnable Tasks and claim a known Task id, but a local Agent
still had to read the projection, choose a candidate externally, then call
`claim task`. That left an avoidable race window and made the command shell
less useful for continuous tool-driven work.

## Decision

1. Phase 3AH adds `ClaimNextOptions` and `ClaimNextResult` to the Engine
   facade.
2. `claim_next_task` uses the existing runnable projection order and selects
   the first candidate that is runnable and unclaimed.
3. The claim write, Session Focus update, Session activity update, and
   provenance Events are persisted in one transaction after revalidating the
   Session target and Branch head.
4. If no unclaimed runnable candidate exists, the operation returns an empty
   selection without writing runtime or history rows.
5. The CLI exposes thin `claim next` over the Engine API.
6. This slice does not define Task integer priority direction, explicit manual
   order, claim takeover, automatic TaskStart, context packet rendering, or
   retry/reselect after a concurrent competing claim.

## Consequences

- A local Agent can now ask WorkVCS for the next runnable item and claim/focus
  it through one Engine command.
- WorkState history remains unchanged because `claim next` is runtime
  coordination state only.

## Implementation Findings

- Previous ADRs intentionally left Task integer priority direction unfixed.
  Claim-next therefore follows the current deterministic runnable projection
  order instead of inventing priority semantics.
