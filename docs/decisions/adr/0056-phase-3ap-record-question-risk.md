# ADR-0056: Phase 3AP Question and Risk Records

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed explicit semantic Record boundary.

## Context

The confirmed V1 Record kinds include Finding, Assumption, Question, Attempt,
ordinary decision, Risk, and Handoff. Phase 3AL through 3AO implemented Finding,
Assumption, Assumption transitions, and ordinary decision Records.

Question and Risk are simple explicit semantic Records for the current slice:
they do not require their own lifecycle transition graph before they can be
useful to a local Agent.

## Decision

1. Phase 3AP extends the Record semantic API with `RecordKind::Question` and
   `RecordKind::Risk`.
2. Question and Risk creation write Records with `status=active`.
3. Both kinds use the existing Record state shape: kind, statement, scope, and
   status.
4. The CLI exposes thin `record question ...` and `record risk ...` commands.
5. This slice does not implement Attempt records, Handoff records, promoted
   Decision entities, Question/Risk transitions, typed causal Record relations,
   or context ranking for Records.

## Consequences

- A local Agent can persist explicit Questions and Risks as versioned WorkState.
- The Record API now covers most simple semantic observations without transcript
  inference.

## Implementation Findings

- No schema or WorkState mapping changes were required.
