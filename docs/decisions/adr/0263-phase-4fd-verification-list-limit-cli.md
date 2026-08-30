# ADR-0263: Phase 4FD Verification List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs verification list` exposes Verification snapshots at a Branch head or
Commit and already supports filtering by target kind, target Entity, and
result. Stores can accumulate many Verification records across repeated manual
and resource-backed checks, so CLI users need bounded output for quick
inspection and scripted reads.

## Decision

`workvcs verification list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Verification
list after the Branch or Commit has been resolved and after any target or
result filters have been applied.

## Consequences

Users can bound Verification discovery output without changing Verification
state, evidence relations, history state, or projection semantics.

This slice does not change the Engine Verification query contract, does not
refresh projections, and does not add schema.
