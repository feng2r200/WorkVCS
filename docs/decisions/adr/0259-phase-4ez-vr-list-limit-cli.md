# ADR-0259: Phase 4EZ Verification Requirement List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs vr list` exposes Verification Requirement snapshots at a Branch head
or Commit and already supports filtering by Acceptance Criterion and local key.
WorkVCS stores can contain many requirements across criteria, so CLI users
need bounded output for quick inspection and scripted reads.

## Decision

`workvcs vr list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Verification
Requirement list after the Branch or Commit has been resolved and after any
Acceptance Criterion or local-key filters have been applied.

## Consequences

Users can bound Verification Requirement discovery output without changing
Acceptance Criterion, Verification Requirement, history, or projection
semantics.

This slice does not change the Engine Verification Requirement query contract,
does not refresh projections, and does not add schema.
