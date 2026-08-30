# ADR-0192: Phase 4CK Task Show And List CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The CLI can create and transition Tasks, and it can list structural Task
relations, but it lacked direct read-only commands for inspecting Task entity
state. This made CLI-only workflows depend on broader replay output for basic
Task status checks.

## Decision

1. Expose the existing internal `tasks_at` helper through Store and Engine as a
   read-only `Engine::tasks_at` facade method.
2. Add `task show` as a thin wrapper around `Engine::task_at`.
3. Add `task list` as a thin wrapper around `Engine::tasks_at`.
4. Allow exactly one query target for each command: `--branch` for current
   branch head or `--commit` for historical state.
5. Render Task description and outcome as canonical JSON values, while keeping
   ids, status, priority, and acceptance criterion counts as stable key/value
   fields.

## Non-Goals

- This slice does not add Task filtering.
- This slice does not change Task lifecycle semantics or claim rules.
- This slice does not materialize a projection table.
- This slice does not alter Verification, Merge, Federation, Runtime, or Store
  mutation behavior.

## Consequences

- CLI users can now inspect and enumerate Task semantic state directly.
- The Task command family has a minimal create / transition / show / list
  workflow surface aligned with Goal and Plan.

## Implementation Findings

- None.
