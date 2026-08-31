# ADR-0398: Phase 4KI CLI Completion Gate Smoke

Status: Accepted
Date: 2026-08-31

## Context

ADR-0397 added a repository-level smoke workflow for the current WorkVCS V0.1
CLI loop. That workflow records a passed Verification and reaches runnable/next
coordination, but it does not prove the user-visible Task completion gate at
the process boundary.

The confirmed domain rules require mandatory Acceptance Criteria to project to
`verified` before a Task can become `done`. Core tests already cover the
Engine-level gate, and current CLI probes confirm the behavior is available:
an unverified mandatory AC blocks `task transition --status done`; a passed
direct Verification can make the AC effective status `verified`; and the
current resource-backed smoke path becomes `verified` after the Verification's
applicability cache records the resource basis as applicable.

## Decision

Phase 4KI extends `scripts/smoke-v0.1-cli-workflow.sh` to cover the mandatory
AC completion gate.

The smoke workflow now proves:

- a Task with an unverified required Acceptance Criterion cannot transition to
  `done` through the CLI;
- `ac status` reports `unverified` before the passed Verification;
- the passed resource-backed Verification plus applicable cache makes
  `ac status` report `verified`;
- the Task can transition to `done` after the Verification;
- post-completion `task show` and `runnable tasks` reflect the completed Task
  state and non-runnable projection.

This slice does not change Engine, Store, schema, Verification closure,
Acceptance Criterion semantics, Task lifecycle semantics, Session/Claim
runtime behavior, or `next` selection. It only makes the existing CLI behavior
part of the repository smoke workflow.

## Consequences

The smoke script now covers the local tool loop through verified Task
completion instead of stopping at runnable selection and claim coordination.
Future completion-related CLI slices can extend the same process-boundary
workflow, but they must keep semantic changes in focused Engine tests and ADRs
rather than using the smoke script as a specification substitute.
