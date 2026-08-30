# ADR-0262: Phase 4FC Resource Observation List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs resource observation-list` exposes Resource Observation snapshots and
already supports filtering by Resource, adapter kind, adapter schema version,
and source Session. Stores can accumulate many observations as adapters sample
external resources, so CLI users need bounded output for quick inspection and
scripted reads.

## Decision

`workvcs resource observation-list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Resource
Observation list after all supplied filters have been applied.

## Consequences

Users can bound Resource Observation discovery output without changing Resource
Observation storage, Resource state, or projection semantics.

This slice does not change the Engine Resource Observation query contract, does
not refresh projections, and does not add schema.
