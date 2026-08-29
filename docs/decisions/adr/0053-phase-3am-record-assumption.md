# ADR-0053: Phase 3AM Assumption Record

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed explicit semantic Record boundary.

## Context

Phase 3AL introduced versioned `Record(kind=finding)` creation. The confirmed
domain model also requires explicit Assumptions, with lifecycle transitions
from `unverified` to `validated` or `invalidated`, and from `validated` to
`invalidated`.

For continued tool progress, the next useful boundary is creating an
Assumption as versioned WorkState before implementing its transition graph.

## Decision

1. Phase 3AM extends the Record semantic API with `RecordKind::Assumption`.
2. Assumption creation writes a `Record(kind=assumption)` state with
   `status=unverified`.
3. Assumption state keeps the same Record state shape as Finding: kind,
   statement, scope, and status.
4. The CLI exposes thin `record assumption STORE --branch <id> --head <id>
   --statement <text> [--scope-json <object>]`.
5. This slice does not implement Assumption validation/invalidation
   transitions, Finding-invalidates-Assumption relations, Attempt records,
   ordinary decision records, Handoff, Risk, Question, or context ranking for
   Records.

## Consequences

- A local Agent can now persist an explicit unverified Assumption instead of
  leaving it in transcript text.
- Later transition slices can update the same Record state through the semantic
  Record API.

## Implementation Findings

- The Phase 3AL Record state shape was sufficient for Assumption creation; only
  the allowed kind/status vocabulary needed expansion.
