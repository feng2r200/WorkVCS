# ADR-0052: Phase 3AL Finding Record

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed explicit semantic Record boundary.

## Context

The confirmed domain model requires explicit semantic records for Agent-known
meaning such as Finding, Assumption, Question, Attempt, ordinary decision,
Risk, and Handoff. These Records participate in Workspace WorkState through the
ordinary Entity/EntityVersion family; schema v0.1 does not define separate
tables for each Record kind.

Earlier Phase 3 slices implemented Goal, Plan, Task, Verification, Evidence,
runtime coordination, context, and next workflow, but did not yet give an
Agent a way to record a simple Finding as versioned WorkState.

## Decision

1. Phase 3AL adds the `record` semantic entity kind.
2. The first supported Record kind is `finding`.
3. A Finding Record state contains kind, statement, scope, and lifecycle status.
4. Finding creation uses the existing semantic Entity transition / Commit / CAS
   path and advances the Branch head.
5. The public generic Entity transition boundary treats `record` as a reserved
   semantic kind, so callers must use the Record API.
6. The CLI exposes thin `record finding STORE --branch <id> --head <id>
   --statement <text> [--scope-json <object>]`.
7. This slice does not implement Assumption, Question, Attempt, ordinary
   decision, Risk, Handoff, Record transitions, promotion to Decision, typed
   causal Record relations, or context ranking for Records.

## Consequences

- A local Agent can now persist explicit Findings without inferring them from a
  transcript.
- Finding history is retained through the same immutable Commit, ChangeSet,
  EntityVersion, and WorkState mechanisms as other semantic entities.

## Implementation Findings

- No schema change was required. The existing Entity/EntityVersion and WorkState
  tables already support the first Record kind.
