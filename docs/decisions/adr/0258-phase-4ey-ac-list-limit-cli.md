# ADR-0258: Phase 4EY Acceptance Criterion List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs ac list` exposes Acceptance Criterion snapshots at a Branch head or
Commit and already supports filtering by owning Task and classification.
WorkVCS stores can contain many criteria for a Task or snapshot, so CLI users
need bounded output for quick inspection and scripted reads.

## Decision

`workvcs ac list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Acceptance
Criterion list after the Branch or Commit has been resolved and after any Task
or classification filters have been applied.

## Consequences

Users can bound Acceptance Criterion discovery output without changing Task,
Acceptance Criterion, history, or projection semantics.

This slice does not change the Engine Acceptance Criterion query contract, does
not refresh projections, and does not add schema.
