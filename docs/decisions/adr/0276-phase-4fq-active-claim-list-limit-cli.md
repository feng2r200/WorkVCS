# ADR-0276: Phase 4FQ Active Claim List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs claim list` exposes active Claim Runtime rows for one Session and
already supports filtering by Task and claim mode. Active claim output can grow
as tooling coordinates parallel work in a Session, so CLI users need bounded
output for quick inspection and scripted reads.

## Decision

`workvcs claim list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered active Claim
list after the Session has been resolved and after all supplied filters have
been applied.

## Consequences

Users can bound active Claim discovery output without changing claim lifecycle,
claim guard behavior, task runtime state, history state, or storage semantics.

This slice does not add historical released-claim listing, does not change
claim acquisition or release, does not change Engine Claim query APIs, and does
not add schema.
