# ADR-0277: Phase 4FR Runnable Task Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs runnable tasks` exposes the current Session's runnable Task projection
and already supports filtering by Task, Task status, and runnable state.
Runnable candidate output can grow as a Workspace accumulates pending Tasks, so
CLI users need bounded output for quick inspection and scripted reads.

## Decision

`workvcs runnable tasks` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered runnable
candidate list after the Session has been resolved and after all supplied
filters have been applied.

## Consequences

Users can bound runnable Task discovery output without changing Task state,
claim coordination, Session focus, `claim next`, `next`, history state, or
storage semantics.

This slice does not change runnable ordering, dependency readiness, claim
selection, Engine runnable query APIs, or schema.
