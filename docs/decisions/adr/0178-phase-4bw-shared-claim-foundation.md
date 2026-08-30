# ADR-0178: Phase 4BW Shared Claim Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The confirmed V1 concurrency model allows either one active exclusive Claim or
one-or-more active shared Claims for the same Workspace, Branch, and Task.
Phase 3F implemented only the exclusive default while preserving the physical
`claim.mode` vocabulary.

The next smallest tool capability is to allow Agents to explicitly request a
shared Claim without changing Task status, WorkState, or the default
exclusive `claim_next_task` behavior.

## Decision

1. `ClaimMode` gains `Shared`.
2. `ClaimTaskOptions::new` keeps the existing default exclusive mode.
3. Callers may explicitly select shared mode through `ClaimTaskOptions::with_mode`.
4. Active Claim-set validation enforces:
   - exclusive Claims reject any existing active Claim for the same Task and
     Branch;
   - shared Claims reject an active exclusive Claim;
   - multiple shared Claims may coexist when they are owned by different
     Sessions;
   - one Session cannot create duplicate active shared Claims for the same
     Task and Branch.
5. Runnable projection renders active shared Claim sets without treating them
   as corruption. A participating shared Session may continue to see the Task
   as runnable; a non-participating Session sees it as claim-blocked.
6. The CLI exposes shared mode through `claim task --mode shared`.
7. `claim_next_task` continues to create default exclusive Claims only.

## Non-Goals

- This slice does not implement claim mode changes.
- This slice does not implement stale-claim takeover, transfer, or force
  provenance.
- This slice does not make shared Claim creation automatic.
- This slice does not relax semantic invariants for terminal or structural
  Task mutations.

## Consequences

- Multiple Agents can explicitly coordinate on one Task as shared participants.
- Existing default exclusive Claim behavior remains source-compatible and
  command-compatible.
- Later slices can add takeover, transfer, and shared-aware terminal-action
  gates without changing the stored Claim shape.

## Implementation Findings

- No schema change was required because schema v0.1 already permits
  `mode = 'shared'`.
- The previous runnable projection assumed at most one active Claim per Task.
  Shared Claim support required interpreting the active Claim set before
  deriving claim-blocked status.
