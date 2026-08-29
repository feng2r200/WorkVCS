# ADR-0061: Phase 3AU Handoff Record Creation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Handoff boundary.

## Context

Confirmed V1 Records include Handoff. The confirmed model says a Session may
create zero or more Handoff Records, each Handoff belongs to exactly one
Workspace and Work Branch, and Session end may recommend but never fabricates or
requires a Handoff.

The remaining confirmed Record kind can be implemented as an explicit semantic
Record operation using the existing Record state shape.

## Decision

1. Phase 3AU adds `RecordKind::Handoff`.
2. Handoff creation writes `Record(kind=handoff)` with `status=active`.
3. Handoff state uses the existing Record state shape: kind, statement, scope,
   and status.
4. Optional focus or context-path bindings are represented in the existing
   `scope` object for this slice; no finer Handoff payload schema is frozen.
5. The CLI exposes `record handoff ...`, and `record list` / `record show`
   support the `handoff` kind.
6. This slice does not modify SessionEnd, generate Handoffs automatically,
   implement cross-Workspace SessionDiff behavior, or add typed Handoff
   relations.

## Consequences

- All currently confirmed simple Record kinds are now writable through the
  semantic Record API.
- Handoff remains an explicit versioned WorkState operation rather than an
  inferred transcript or SessionEnd side effect.

## Implementation Findings

- The existing Record state shape is sufficient for explicit Handoff creation.
