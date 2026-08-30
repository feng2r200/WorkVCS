# ADR-0254: Phase 4EU Branch List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs branch list` exposes Branch discovery for one Workspace and already
supports filtering by Branch name and lifecycle state. Stores with repeated
local work may contain many branches, so CLI users need bounded output for
quick inspection.

## Decision

`workvcs branch list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Branch list
after any name or lifecycle-state filters have been applied.

## Consequences

Users can bound Branch discovery output without changing Workspace, Branch, or
history state.

This slice does not change the Engine Branch list contract, does not refresh
projections, and does not add schema.
