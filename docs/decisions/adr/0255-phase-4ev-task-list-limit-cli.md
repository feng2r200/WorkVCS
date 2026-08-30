# ADR-0255: Phase 4EV Task List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs task list` exposes Task snapshots at a Branch head or Commit and
already supports status filtering. Real workspaces can accumulate many Tasks,
so CLI users need bounded output for quick inspection and scripted reads.

## Decision

`workvcs task list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Task list
after the Branch or Commit has been resolved and after any status filter has
been applied.

## Consequences

Users can bound Task discovery output without changing Task state, history
state, or projection semantics.

This slice does not change the Engine Task query contract, does not refresh
projections, and does not add schema.
