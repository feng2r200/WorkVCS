# ADR-0180: Phase 4BY Shared Next Work

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0179 added explicit shared mode to `claim_next_task` and the CLI
`claim next` command. The top-level `next` workflow is the more useful tool
entrypoint for an Agent because it claims work and returns the resulting
Context overview in one call.

## Decision

1. `NextWorkOptions::new` continues to default to `ClaimMode::Exclusive`.
2. Callers may explicitly choose shared mode through
   `NextWorkOptions::with_mode(ClaimMode::Shared)`.
3. `next_work` delegates selection and Claim creation to `claim_next_task`
   using the requested mode, then returns the post-claim Context overview as
   before.
4. CLI exposes the mode through `workvcs next --mode shared`; omitted mode
   remains exclusive.

## Non-Goals

- This slice does not implement Claim mode changes.
- This slice does not implement takeover, transfer, force, or stale Session
  recovery.
- This slice does not change Context packet ranking, budgeting, or rendering.
- This slice does not make shared Claim creation automatic.

## Consequences

- Agents can atomically join shared work and immediately receive the updated
  Context overview through the primary `next` command.
- Existing `workvcs next` callers remain command-compatible and keep exclusive
  selection semantics by default.

## Implementation Findings

- No schema or Context projection changes were required. The existing
  post-claim Context read already reflects shared Claim coordination.
