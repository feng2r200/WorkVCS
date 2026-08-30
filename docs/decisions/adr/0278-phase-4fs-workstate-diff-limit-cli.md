# ADR-0278: Phase 4FS WorkState Diff Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs diff` exposes entity and relation membership changes between two
WorkState targets and already supports filtering by target kind, change kind,
entity id, and relation id. Large branch comparisons can produce many changes,
so CLI users need bounded output for quick inspection and scripted reads.

## Decision

`workvcs diff` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered change list
after both WorkState targets have been resolved and after all supplied filters
have been applied.

Because existing rendering prints entity changes before relation changes, the
limit is applied to that rendered order: entity changes consume the limit first,
and relation changes receive any remaining capacity. When callers also pass
`--target-kind entity` or `--target-kind relation`, the limit applies to that
single selected change family.

## Consequences

Users can bound WorkState diff output without changing WorkState diff
semantics, replay, history state, branch projection, or storage semantics.

This slice does not add pagination, does not compute semantic field-level
diffs, does not change Engine diff query APIs, and does not add schema.
