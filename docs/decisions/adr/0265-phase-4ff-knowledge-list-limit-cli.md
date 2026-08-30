# ADR-0265: Phase 4FF Knowledge List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs knowledge list` exposes Knowledge snapshots at a Branch head or Commit
and already supports filtering by status, scope, and statement text. Stores can
accumulate many Knowledge records across repeated analysis and review, so CLI
users need bounded output for quick inspection and scripted reads.

## Decision

`workvcs knowledge list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Knowledge
list after the Branch or Commit has been resolved and after all supplied
filters have been applied.

## Consequences

Users can bound Knowledge discovery output without changing Knowledge state,
history state, or projection semantics.

This slice does not change the Engine Knowledge query contract, does not
refresh projections, and does not add schema.
