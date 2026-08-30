# ADR-0295: Phase 4GJ Why Relation Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs why` renders relation edges with relation kind and direction. The
explanation engine can return multiple relation families for a subject, but the
CLI could not narrow the rendered edge set.

## Decision

`workvcs why` accepts:

- `--relation-kind <KIND>`
- `--direction <incoming|outgoing>`

The CLI obtains the normal `WhyQueryResult` through the Engine facade, filters
`relation_edges` before rendering, and leaves the target, subject, and deferred
family metadata unchanged.

## Consequences

Users can focus `why` explanations on a specific relation family or direction
without changing the explanation engine, schema, or query contracts.
