# ADR-0268: Phase 4FI Knowledge Relation List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs knowledge relation-list` exposes relations between Knowledge records
at a Branch head or Commit and already supports filtering by replacement or
prior Knowledge. Stores can accumulate many Knowledge supersession edges as
analysis evolves, so CLI users need bounded output for quick inspection and
scripted reads.

## Decision

`workvcs knowledge relation-list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Knowledge
Relation list after the Branch or Commit has been resolved and after all
supplied filters have been applied.

## Consequences

Users can bound Knowledge Relation discovery output without changing Knowledge
Relation state, Knowledge state, history state, or projection semantics.

This slice does not change the Engine Knowledge Relation query contract, does
not refresh projections, and does not add schema.
