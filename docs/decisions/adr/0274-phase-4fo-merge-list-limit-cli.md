# ADR-0274: Phase 4FO Merge List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs merge list` exposes merge attempts for one Workspace and already
supports filtering by target Branch, inclusion of closed attempts, runtime
state, and outcome. Merge attempt output can grow as operators run repeated
branch reconciliation workflows, so CLI users need bounded output for quick
inspection and scripted reads.

## Decision

`workvcs merge list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Merge list
after the Workspace and optional target Branch have been resolved and after all
supplied filters have been applied.

## Consequences

Users can bound Merge discovery output without changing merge attempts, merge
items, merge lifecycle state, history state, or storage semantics.

This slice does not change merge start, abort, continue, resolution freeze,
Engine merge query APIs, or schema.
