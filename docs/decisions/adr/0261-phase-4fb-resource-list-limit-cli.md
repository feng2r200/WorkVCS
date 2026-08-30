# ADR-0261: Phase 4FB Resource List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs resource list` exposes Resources and already supports filtering by
Resource kind, binding presence, and Workspace association. Stores can
accumulate many Resources as adapters and workspace links are recorded, so CLI
users need bounded output for quick inspection and scripted reads.

## Decision

`workvcs resource list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Resource
list after kind, binding, and Workspace filters have been applied.

## Consequences

Users can bound Resource discovery output without changing Resource storage,
Workspace association state, or projection semantics.

This slice does not change the Engine Resource query contract, does not refresh
projections, and does not add schema.
