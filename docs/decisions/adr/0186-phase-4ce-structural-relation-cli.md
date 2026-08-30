# ADR-0186: Phase 4CE Structural Relation CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Task scheduling relations and primary containment relations already exist as
Engine semantic APIs. ADR-0185 added optional actor Session guard enforcement to
those core operations. The CLI needs a thin user-facing route for creating those
relations without introducing another business authority.

## Decision

1. Add `task depends-on` as a thin wrapper around
   `TaskSchedulingRelationCreateOptions::depends_on`.
2. Add `task ordered-before` as a thin wrapper around
   `TaskSchedulingRelationCreateOptions::ordered_before`.
3. Add `task contain` as a thin wrapper around
   `PrimaryContainmentCreateOptions::new`.
4. Each command accepts optional `--session`; when supplied, it is forwarded as
   the actor Session so Store-level claim guard enforcement applies.
5. Output remains stable key/value text matching the existing CLI rendering
   style.

## Non-Goals

- This slice does not add Plan or Goal creation CLI commands.
- This slice does not add structural relation list/show CLI commands.
- This slice does not change core relation semantics or Claim guard rules.
- This slice does not introduce a separate CLI-side validation authority.

## Consequences

- Users can create dependency, manual-order, and Task containment relations
  through the CLI while reusing Engine enforcement.
- Existing scripts can parse relation create results with the same key/value
  format used by other WorkVCS commands.

## Implementation Findings

- Because the CLI still has no Plan/Goal creation commands, the first CLI
  containment path is Task-to-Task containment. Plan/Goal containment remains
  available through the Engine API.
