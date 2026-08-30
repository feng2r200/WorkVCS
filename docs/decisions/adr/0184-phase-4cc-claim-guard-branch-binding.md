# ADR-0184: Phase 4CC Claim Guard Branch Binding

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0183 applied Claim guard enforcement to terminal Task transitions when a
caller provides an actor Session. The guard evaluates the actor Session's active
Workspace, Branch, and head. Store-level enforcement must therefore bind that
guard result back to the exact Branch and expected head targeted by the
historical transition.

## Decision

1. `TaskTransitionOptions` exposes read-only getters for target Branch and
   expected head.
2. Terminal Task transition enforcement rejects an actor Session whose active
   Branch does not match the transition target Branch.
3. Terminal Task transition enforcement reports `BranchHeadConflict` when the
   actor Session's active Branch head has moved away from the transition's
   expected head.
4. Both rejection paths leave history rows and Branch head unchanged.

## Non-Goals

- This slice does not change Claim guard ownership rules.
- This slice does not make actor Session mandatory for legacy transition
  callers.
- This slice does not apply structural mutation guard enforcement.

## Consequences

- Actor Session based terminal Task enforcement cannot authorize work on a
  different Branch or a stale expected head.
- Later structural enforcement can reuse the same binding rule.

## Implementation Findings

- The binding check belongs in Store-level orchestration because it combines
  runtime Session state with the historical operation target while keeping the
  history module independent from runtime coordination.
