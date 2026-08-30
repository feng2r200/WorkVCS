# ADR-0297: Phase 4GL Why Relation Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs why` can now filter relation edges by relation kind, direction, endpoint
id, and endpoint kind. Large explanation neighborhoods still need a bounded CLI
view for manual inspection.

## Decision

`workvcs why` accepts `--relation-limit <N>`.

The CLI rejects `0`, applies all relation filters first, then truncates the
rendered `relation_edges` vector to `N`. The Engine facade, schema, and output
field names remain unchanged.

## Consequences

Users can inspect a bounded prefix of deterministic `why` relation output
without introducing pagination or expanding the Engine query contract.
