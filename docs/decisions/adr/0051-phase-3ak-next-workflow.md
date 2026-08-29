# ADR-0051: Phase 3AK Next Workflow

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed atomic claim-next / Context Resolver boundary.

## Context

Phase 3AH implemented `claim_next_task`, which selects the first runnable and
unclaimed Task from the deterministic runnable projection, creates the Claim,
and focuses the Session in one write transaction. Phase 3AJ added a read-only
context overview for the active Session anchor.

The confirmed product model also distinguishes `next` from `context`: `context`
reports current state without choosing work, while `next` is the Agent workflow
for selecting and claiming the next runnable item.

## Decision

1. Phase 3AK adds `Engine::next_work(session_id)`.
2. `next_work` invokes the existing atomic claim-next operation and then returns
   the post-claim `ContextOverview`.
3. If no runnable unclaimed candidate exists, `next_work` returns no selection
   and still returns the current context overview.
4. The CLI exposes thin `next STORE --session <id>` output in the existing
   line-oriented `key=value` style.
5. This slice does not implement automatic TaskStart, manual ordering,
   candidate takeover, retry-after-conflict selection, full context packet
   rendering, or budgeted context trimming.

## Consequences

- A local Agent can ask WorkVCS for the next actionable item and immediately
  receive the post-claim Session context needed to continue.
- `context` remains read-only; `next` is the explicit mutating command.

## Implementation Findings

- Phase 3AH intentionally left the Task integer priority direction unfixed, so
  `next_work` inherits the current runnable projection order rather than
  defining a new priority policy.
