# ADR-0050: Phase 3AJ Context Overview

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Context Resolver / Runtime Coordination boundary.

## Context

The confirmed product and engine design require a deterministic `context`
resolver. Earlier Phase 3 slices implemented Session runtime, Focus, runnable
Task projection, claim-next, Branch fork/switch, and Branch listing, but a local
Agent still lacked one read-only command that described the current Session
anchor and work candidates together.

The full resolver also includes structural context, execution constraints,
causal neighborhood, profile filtering, and whole-item budget trimming. Those
rules are confirmed as a deterministic direction, but not all item families are
implemented yet.

## Decision

1. Phase 3AJ adds `Engine::context_overview(session_id)`.
2. The operation returns an active `SessionSnapshot`, the active `BranchHead`,
   and the existing `RunnableTasksProjection` for the same anchor.
3. The operation is read-only and writes no runtime or WorkState rows.
4. The operation rejects ended Sessions instead of fabricating inactive context.
5. The CLI exposes thin `context STORE --session <id>` output in the existing
   line-oriented `key=value` style.
6. This slice does not implement full context profile selection, budget
   trimming, knowledge-space inclusion policy, causal ranking, context packet
   persistence, or atomic claim-next context rendering.

## Consequences

- A local Agent can inspect the active Workspace, Branch head, Focus, Context
  Set, and runnable candidates through one command before choosing the next
  action.
- The slice keeps `context` from silently becoming `next`: it reports current
  candidates but does not claim or select a Task.

## Implementation Findings

- The existing `SessionSnapshot`, `BranchHead`, and `RunnableTasksProjection`
  types already provide enough data for a first context overview without schema
  change.
