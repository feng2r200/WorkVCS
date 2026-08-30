# ADR-0271: Phase 4FL Task Containment List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs task containment-list` exposes primary containment relations at a
Branch head or Commit and already supports filtering by parent, child,
parent kind, and child kind. Goal, Plan, and Task hierarchy output can grow as
tooling records richer workspace structure, so CLI users need bounded output
for quick inspection and scripted reads.

## Decision

`workvcs task containment-list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Primary
Containment Relation list after the Branch or Commit has been resolved and
after all supplied filters have been applied.

## Consequences

Users can bound Primary Containment Relation discovery output without changing
Goal state, Plan state, Task state, containment relation state, history state,
or storage semantics.

This slice does not add pagination, does not traverse hierarchy descendants,
does not change Engine containment query APIs, and does not add schema.
