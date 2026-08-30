# ADR-0245: Phase 4EL Runnable Task Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs runnable tasks` exposes the current session's runnable task projection,
including candidate task id, task status, runnable state, dependency readiness,
and claim coordination. Operators need to narrow this projection without
invoking `next` or changing claim state.

The engine already returns the full runnable projection for a session. This
slice does not change runnable ordering, dependency readiness, claim selection,
or session context behavior.

## Decision

`workvcs runnable tasks` accepts:

- `--task <TASK_ENTITY_ID>`
- `--status <pending|in_progress|blocked|done>`
- `--runnable <true|false>`

All filters are CLI-side filters over the existing `RunnableTasksProjection`.
When multiple filters are present, all must match.

## Consequences

The CLI can inspect specific runnable candidates and status subsets without
mutating task, claim, or session state.

`workvcs next` remains the authoritative command for choosing and claiming the
next task; this slice only narrows read output.
