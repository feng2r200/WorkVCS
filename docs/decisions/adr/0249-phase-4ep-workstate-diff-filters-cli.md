# ADR-0249: Phase 4EP WorkState Diff Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

ADR-0212 added `workvcs diff` as a read-only CLI wrapper over
`Engine::diff`. That command renders all entity and relation membership changes
between two WorkState targets, but intentionally left filtering out of the
initial slice.

CLI users often need to inspect only entity changes, relation changes, or one
change kind while comparing branch and commit state.

## Decision

`workvcs diff` accepts optional:

- `--target-kind <entity|relation>`
- `--change-kind <added|removed|updated>`

The CLI computes the full diff through `Engine::diff`, then filters the returned
entity and relation change vectors before rendering. Multiple filters may be
combined and all supplied filters must match.

## Consequences

Users can narrow WorkState diff output without writing custom post-processing.

This slice does not change WorkState diff semantics, does not inspect semantic
entity payloads, does not refresh branch projections, and does not change the
Engine diff contract.
