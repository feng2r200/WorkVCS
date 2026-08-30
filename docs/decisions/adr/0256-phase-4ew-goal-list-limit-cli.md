# ADR-0256: Phase 4EW Goal List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs goal list` exposes Goal snapshots at a Branch head or Commit and
already supports status filtering. As WorkVCS stores accumulate long-running
goal history, CLI users need bounded output for quick inspection and scripted
reads.

## Decision

`workvcs goal list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Goal list
after the Branch or Commit has been resolved and after any status filter has
been applied.

## Consequences

Users can bound Goal discovery output without changing Goal state, history
state, or projection semantics.

This slice does not change the Engine Goal query contract, does not refresh
projections, and does not add schema.
