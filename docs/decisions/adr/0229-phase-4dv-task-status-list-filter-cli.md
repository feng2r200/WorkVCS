# ADR-0229: Phase 4DV Task Status List Filter CLI

Status: Accepted

Date: 2026-08-30

## Context

`workvcs task list` can enumerate Task state at a Branch head or historical
Commit, but it cannot narrow the result by lifecycle status. Status filtering is
one of the most common operator queries when deciding what work is pending,
blocked, active, or terminal.

ADR-0192 intentionally kept filtering out of the first Task readback slice. This
slice adds only a read-side status filter and does not change Task lifecycle
mutation semantics.

## Decision

1. Add `--status STATUS` to `workvcs task list`.
2. Resolve the existing Branch/Commit target exactly as before.
3. Load Task snapshots through the existing Engine facade and filter by
   `TaskStatus` before rendering.
4. Keep `task transition` status parsing unchanged.
5. Allow `superseded` as a read-side filter value even though entering
   `superseded` remains deferred to supersession-aware mutation semantics.

## Non-Goals

- This slice does not add Task kind, priority, text, or containment filters.
- This slice does not change Task lifecycle transitions.
- This slice does not make `superseded` writable through `task transition`.
- This slice does not change runnable or `next` selection.

## Consequences

- CLI users can ask for Task lists by confirmed lifecycle status.
- Imported or future superseded Task states remain queryable without opening the
  supersession write path.
- The command remains a thin read-only wrapper over existing semantic snapshots.
