# ADR-0299: Phase 4GN Why Filter Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs why` supports CLI-side filters over the relation edges returned by the
Engine facade. Before this slice, an unsupported filter value behaved like a
valid filter with no matches, which made typos difficult to distinguish from an
empty explanation neighborhood.

## Decision

The CLI validates `workvcs why` filter vocabularies before opening the target
store:

- relation kind
- relation direction
- endpoint kind
- endpoint entity kind
- relation limit

Unsupported vocabulary values return `QueryInvalid`. `--relation-limit 0`
continues the existing bounded-query rule that limits must be positive.

## Consequences

Manual `why` usage fails fast on misspelled filters while keeping all filtering
in the CLI layer. The Engine facade, schema, and rendered output field names do
not change.
