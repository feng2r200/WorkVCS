# ADR-0298: Phase 4GM Why Endpoint Entity Kind Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs why` can filter relation edges by endpoint id and endpoint kind.
Entity endpoints also carry an entity kind such as goal, task, record, or
knowledge, which is useful when a relation neighborhood mixes semantic domains.

## Decision

`workvcs why` accepts:

- `--source-entity-kind <KIND>`
- `--target-entity-kind <KIND>`

The CLI applies these filters after endpoint-kind filters and before relation
limits. Non-entity endpoints do not match entity-kind filters. The Engine
facade, schema, and output field names remain unchanged.

## Consequences

Users can narrow explanation output to entity endpoint categories without
expanding the `why` query contract or changing stored relation semantics.
