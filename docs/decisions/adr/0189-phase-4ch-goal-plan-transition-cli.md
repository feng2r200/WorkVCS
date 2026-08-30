# ADR-0189: Phase 4CH Goal And Plan Transition CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Goal and Plan lifecycle transitions already exist in the Engine, and Phase 4CG
added CLI creation commands for both semantic entity families. CLI-only
workflows could create a Goal/Plan hierarchy but still could not move those
entities through their confirmed lifecycle states without writing Rust tests or
calling the Engine directly.

## Decision

1. Add `goal achieve`, `goal abandon`, and `goal reopen` as thin wrappers around
   `GoalTransitionOptions`.
2. Add `plan complete`, `plan abandon`, and `plan reopen` as thin wrappers
   around `PlanTransitionOptions`.
3. Require explicit branch, expected head, entity id, and expected entity
   version id for every transition command.
4. Keep transition rationale requirements aligned with the existing Engine
   contracts: Goal terminal/reopen transitions require `--rationale`; Plan
   abandon/reopen require `--rationale`; Plan completion accepts optional
   `--completion-rationale`.
5. Render transition results with stable key/value fields covering commit,
   changeset, operation, previous/new entity versions, state digest, WorkState
   digest, and previous/new statuses.

## Non-Goals

- This slice does not add Goal or Plan list/show commands.
- This slice does not alter Goal or Plan lifecycle semantics.
- This slice does not add CLI-side semantic validation beyond argument parsing.
- This slice does not implement Task, Runtime, Verification, Merge, Federation,
  or Store behavior.

## Consequences

- CLI users can now create and transition Goal and Plan entities through their
  confirmed lifecycle states using only Engine-backed commands.
- Later read-only query slices can inspect the lifecycle history without
  changing these command contracts.

## Implementation Findings

- None.
