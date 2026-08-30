# ADR-0272: Phase 4FM Structural Reference List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs reference list` exposes Structural Reference relations at a Branch head
or Commit and already supports filtering by referrer, target, referrer kind, and
target kind. Reference output can grow as tools connect Goal, Plan, and Task
context, so CLI users need bounded output for quick inspection and scripted
reads.

## Decision

`workvcs reference list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Structural
Reference list after the Branch or Commit has been resolved and after all
supplied filters have been applied.

## Consequences

Users can bound Structural Reference discovery output without changing Goal
state, Plan state, Task state, reference relation state, history state, or
storage semantics.

This slice does not add pagination, does not traverse structural neighborhoods,
does not change Engine reference query APIs, and does not add schema.
