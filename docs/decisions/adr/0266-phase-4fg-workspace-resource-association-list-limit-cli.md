# ADR-0266: Phase 4FG Workspace Resource Association List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs resource workspace-association-list` exposes Resource associations for
a Workspace and already supports filtering by Resource. Workspaces can be
linked to many Resources over time, so CLI users need bounded output for quick
inspection and scripted reads.

## Decision

`workvcs resource workspace-association-list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Workspace
Resource Association list after the Workspace has been selected and after any
Resource filter has been applied.

## Consequences

Users can bound Workspace Resource Association discovery output without
changing Workspace, Resource, association, or projection semantics.

This slice does not change the Engine Workspace Resource Association query
contract, does not refresh projections, and does not add schema.
