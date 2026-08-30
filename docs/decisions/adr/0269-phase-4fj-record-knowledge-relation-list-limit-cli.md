# ADR-0269: Phase 4FJ Record Knowledge Relation List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs record knowledge-relation-list` exposes Record-to-Knowledge relations
at a Branch head or Commit and already supports filtering by relation type,
source Record, and target Knowledge. Stores can accumulate many Record evidence
edges over time, so CLI users need bounded output for quick inspection and
scripted reads.

## Decision

`workvcs record knowledge-relation-list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered
Record-to-Knowledge Relation list after the Branch or Commit has been resolved
and after all supplied filters have been applied.

## Consequences

Users can bound Record-to-Knowledge Relation discovery output without changing
Record state, Knowledge state, relation state, history state, or projection
semantics.

This slice does not change the Engine Record-to-Knowledge Relation query
contract, does not refresh projections, and does not add schema.
