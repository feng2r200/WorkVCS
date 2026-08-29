# ADR-0054: Phase 3AN Assumption Transition

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed semantic Record lifecycle boundary.

## Context

Phase 3AM added creation of `Record(kind=assumption)` with initial
`status=unverified`. The confirmed domain model specifies the Assumption
transition graph:

```text
unverified -> validated
unverified -> invalidated
validated   -> invalidated
```

An invalidated Assumption is not ordinarily revalidated.

## Decision

1. Phase 3AN adds semantic Record transition support for Assumptions.
2. The transition operation uses the existing Entity update / Commit / CAS
   path and advances the Branch head.
3. Transitions require a non-empty rationale.
4. Finding Records do not use the Assumption lifecycle.
5. The CLI exposes thin `record assumption-status STORE --branch <id>
   --head <id> --record <id> --record-version <id> --status
   validated|invalidated --rationale <text>`.
6. This slice does not implement Finding-invalidates-Assumption relations,
   superseding Assumptions, Attempt records, ordinary decision records, Handoff,
   Risk, Question, or context ranking for Records.

## Consequences

- A local Agent can now validate or invalidate an explicit Assumption with
  versioned WorkState history.
- The confirmed non-revalidation rule is enforced by the semantic API.

## Implementation Findings

- The existing Record canonical state shape remains sufficient; the lifecycle
  vocabulary and transition validator were the only required state-machine
  additions.
