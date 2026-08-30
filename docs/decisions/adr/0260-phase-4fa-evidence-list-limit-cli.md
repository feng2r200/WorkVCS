# ADR-0260: Phase 4FA Evidence List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs evidence list` exposes captured Evidence and already supports
filtering by Evidence kind and source Session. Stores can accumulate many
evidence records during verification and manual review, so CLI users need
bounded output for quick inspection and scripted reads.

## Decision

`workvcs evidence list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Evidence
list after kind and source-session filters have been applied.

## Consequences

Users can bound Evidence discovery output without changing Evidence storage,
history state, or projection semantics.

This slice does not change the Engine Evidence query contract, does not refresh
projections, and does not add schema.
