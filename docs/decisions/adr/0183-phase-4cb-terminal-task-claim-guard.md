# ADR-0183: Phase 4CB Terminal Task Claim Guard

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0182 introduced a reusable Claim guard decision surface for protected Task
work. The next smallest enforcement step is to apply that guard to terminal
Task lifecycle transitions when the caller provides an actor Session.

## Decision

1. `TaskTransitionOptions` gains optional actor Session binding through
   `with_actor_session`.
2. `TaskStatus::is_terminal` is public so Store-level orchestration can decide
   when Claim protection applies without duplicating the terminal status set.
3. `Store::transition_task` invokes `task_claim_guard` before terminal Task
   transitions when `actor_session_id` is present.
4. A blocked guard decision returns `ClaimInvalid` and leaves history rows and
   Branch head unchanged.
5. CLI `task transition` gains optional `--session` to pass the actor Session.

## Non-Goals

- This slice does not make actor Session mandatory for every terminal
  transition.
- This slice does not apply Claim guard enforcement to structural relation
  writers.
- This slice does not implement transfer, force provenance, takeover, or stale
  Session recovery.

## Consequences

- Tool callers can opt into concrete Claim enforcement for terminal Task
  transitions without changing the older source-compatible transition path.
- The next enforcement slice can reuse the same guard for structural Task
  mutation surfaces.

## Implementation Findings

- Store-level orchestration is the narrowest place to combine runtime Claim
  state with historical Task transitions while preserving the existing history
  module boundary.
