# ADR-0188: Phase 4CG Goal And Plan Create CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The Engine already supports Goal and Plan creation, and structural containment
supports Goal, Plan, and Task endpoints. The CLI could create Tasks and
structural Task relations, but it lacked direct Goal/Plan creation commands,
which made CLI-only hierarchy construction incomplete.

## Decision

1. Add `goal create` as a thin wrapper around `GoalCreateOptions::new`.
2. Add `plan create` as a thin wrapper around `PlanCreateOptions::new`.
3. `plan create` accepts repeatable `--constraint` values and forwards them to
   `PlanCreateOptions::with_constraints`.
4. Render create results with stable key/value fields matching existing CLI
   conventions.
5. Verify the CLI can create a Goal -> Plan -> Task containment hierarchy using
   only Engine-backed commands.

## Non-Goals

- This slice does not add Goal or Plan transition commands.
- This slice does not add Goal or Plan list/show commands.
- This slice does not add CLI-side hierarchy validation.
- This slice does not change Goal, Plan, or containment semantics.

## Consequences

- CLI users can build the basic Goal/Plan/Task structure required by the
  confirmed semantic model.
- Later lifecycle and query CLI slices can reuse the new top-level command
  families.

## Implementation Findings

- `task contain` can already serve mixed Goal/Plan/Task containment because it
  delegates to the generic `PrimaryContainmentCreateOptions` Engine API.
