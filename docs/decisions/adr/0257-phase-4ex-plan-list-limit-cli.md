# ADR-0257: Phase 4EX Plan List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs plan list` exposes Plan snapshots at a Branch head or Commit and
already supports status filtering. WorkVCS stores can contain many Plans across
ongoing and completed work, so CLI users need bounded output for quick
inspection and scripted reads.

## Decision

`workvcs plan list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Plan list
after the Branch or Commit has been resolved and after any status filter has
been applied.

## Consequences

Users can bound Plan discovery output without changing Plan state, history
state, or projection semantics.

This slice does not change the Engine Plan query contract, does not refresh
projections, and does not add schema.
