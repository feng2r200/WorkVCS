# ADR-0055: Phase 3AO Ordinary Decision Record

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed explicit semantic Record / Decision promotion boundary.

## Context

The confirmed model distinguishes an ordinary decision Record from a promoted
Decision. An ordinary decision is recorded as `Record(kind=decision)`. Promotion
to a full Decision is explicit and later change supersedes rather than rewrites
the promoted Decision.

Phase 3AL and Phase 3AM/3AN implemented Finding and Assumption Records. The
next minimal tool capability is to persist an ordinary decision without opening
the full promoted Decision state machine.

## Decision

1. Phase 3AO extends the Record semantic API with `RecordKind::Decision`.
2. Ordinary decision creation writes `Record(kind=decision)` with
   `status=active`.
3. The state shape remains kind, statement, scope, and status.
4. The CLI exposes thin `record decision STORE --branch <id> --head <id>
   --statement <text> [--scope-json <object>]`.
5. This slice does not implement promoted Decision entities, Decision
   supersession, promotion links, decision conflict review, Attempt records,
   Risk, Question, Handoff, or context ranking for Records.

## Consequences

- A local Agent can persist an explicit ordinary decision as versioned WorkState.
- Later promoted Decision work can preserve this origin instead of erasing it.

## Implementation Findings

- The existing Record state shape supports ordinary decision creation without
  schema change.
