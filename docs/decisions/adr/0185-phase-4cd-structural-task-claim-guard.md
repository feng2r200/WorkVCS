# ADR-0185: Phase 4CD Structural Task Claim Guard

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0183 and ADR-0184 protected terminal Task transitions with actor Session
claim guard enforcement and branch/head binding. Structural Task mutations must
use the same runtime authority when a caller supplies an actor Session, because
Task scheduling and Task containment changes affect the runnable structure of a
Task.

## Decision

1. `TaskSchedulingRelationCreateOptions` accepts an optional actor Session.
2. `PrimaryContainmentCreateOptions` accepts an optional actor Session.
3. Task scheduling relation creation checks `StructuralTaskMutation` guard
   access for both Task endpoints when an actor Session is provided.
4. Primary containment creation checks `StructuralTaskMutation` guard access for
   each endpoint that is a Task at the target commit when an actor Session is
   provided.
5. Store-level enforcement binds the actor Session's active Branch and head to
   the operation Branch and expected head before applying structural guards.
6. Guard rejections leave history rows and Branch head unchanged.

## Non-Goals

- This slice does not make actor Session mandatory for legacy structural
  callers.
- This slice does not add new CLI commands for Task scheduling relation or
  primary containment creation.
- This slice does not apply Task claim rules to Goal-only or Plan-only
  containment changes.
- This slice does not change Claim ownership or shared-claim rules.

## Consequences

- Actor Session based structural Task mutations cannot bypass an active
  exclusive/shared Claim on affected Task endpoints.
- Existing semantic callers without actor Session keep their current behavior.
- Later CLI slices can expose structural creation commands without changing the
  core guard semantics.

## Implementation Findings

- Primary containment is mixed Goal/Plan/Task structure, so the structural Task
  guard must be endpoint-sensitive rather than blanket-applied to every
  containment endpoint.
