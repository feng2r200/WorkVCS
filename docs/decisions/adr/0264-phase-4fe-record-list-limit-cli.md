# ADR-0264: Phase 4FE Record List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs record list` exposes Record snapshots at a Branch head or Commit and
already supports filtering by kind, status, scope, and statement text. Stores
can accumulate many findings, assumptions, claims, and decisions, so CLI users
need bounded output for quick inspection and scripted reads.

## Decision

`workvcs record list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Record list
after the Branch or Commit has been resolved and after all supplied filters
have been applied.

## Consequences

Users can bound Record discovery output without changing Record state, history
state, or projection semantics.

This slice does not change the Engine Record query contract, does not refresh
projections, and does not add schema.
