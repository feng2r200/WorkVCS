# ADR-0273: Phase 4FN Checkpoint List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs checkpoint list` exposes stored checkpoints for one Commit and already
supports filtering by usability state and content digest. Checkpoint history can
grow as tools create validation and recovery points, so CLI users need bounded
output for quick inspection and scripted reads.

## Decision

`workvcs checkpoint list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Checkpoint
list after the Commit has been resolved and after all supplied filters have
been applied.

## Consequences

Users can bound Checkpoint discovery output without changing checkpoint content,
checkpoint validation, history state, or storage semantics.

This slice does not change checkpoint creation, latest-checkpoint selection,
checkpoint validation, Engine checkpoint query APIs, or schema.
