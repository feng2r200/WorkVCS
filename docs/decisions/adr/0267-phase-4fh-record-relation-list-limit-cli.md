# ADR-0267: Phase 4FH Record Relation List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs record relation-list` exposes relations between Records at a Branch
head or Commit and already supports filtering by relation type, label, source
Record, and target Record. Stores can accumulate many Record relations as
findings, assumptions, questions, risks, and decisions are linked, so CLI users
need bounded output for quick inspection and scripted reads.

## Decision

`workvcs record relation-list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Record
Relation list after the Branch or Commit has been resolved and after all
supplied filters have been applied.

## Consequences

Users can bound Record Relation discovery output without changing Record
Relation state, Record state, history state, or projection semantics.

This slice does not change the Engine Record Relation query contract, does not
refresh projections, and does not add schema.
