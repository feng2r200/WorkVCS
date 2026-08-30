# ADR-0368: Phase 4JE Why Relation Edge Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs why` is the CLI surface for provenance and explanation queries. It
already supports relation-kind, direction, endpoint, endpoint-kind,
endpoint-entity-kind, and relation-limit filters. Acceptance scripts still had
to parse `relation_edges` externally to assert the expected explanation shape.

## Decision

The CLI adds `workvcs why --expected-relation-edges COUNT`.

The command applies existing filters and `--relation-limit` first, renders the
same output, and appends `relation_edges_match_expected=true` when the final
rendered relation edge count equals the supplied expectation. A mismatch returns
`QueryInvalid`.

## Consequences

Why/provenance acceptance scripts can fail fast without external count parsing.
This does not change the `why` Engine query, relation family semantics,
deferred relation reporting, filtering behavior, or storage schema.
