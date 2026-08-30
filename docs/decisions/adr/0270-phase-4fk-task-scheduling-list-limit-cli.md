# ADR-0270: Phase 4FK Task Scheduling List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs task scheduling-list` exposes Task scheduling relations at a Branch
head or Commit and already supports filtering by relation type, source Task,
and target Task. Dependency and manual-order graphs can grow as a workspace is
planned, so CLI users need bounded output for quick inspection and scripted
reads.

## Decision

`workvcs task scheduling-list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Task
Scheduling Relation list after the Branch or Commit has been resolved and after
all supplied filters have been applied.

## Consequences

Users can bound Task Scheduling Relation discovery output without changing Task
state, scheduling relation state, runnable-task projection, history state, or
storage semantics.

This slice does not add pagination, does not traverse dependency graphs, does
not change Engine scheduling query APIs, and does not add schema.
