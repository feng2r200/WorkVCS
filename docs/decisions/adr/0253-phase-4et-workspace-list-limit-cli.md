# ADR-0253: Phase 4ET Workspace List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs workspace list` exposes Workspace discovery for CLI-only workflows and
already supports exact display-name filtering. As local Stores accumulate
multiple Workspaces, users need a bounded list output for quick inspection.

## Decision

`workvcs workspace list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Workspace
list after any display-name filter has been applied.

## Consequences

Users can bound Workspace discovery output without changing Store contents.

This slice does not change the Engine Workspace list contract, does not add
schema, and does not make Workspace a WorkState member.
