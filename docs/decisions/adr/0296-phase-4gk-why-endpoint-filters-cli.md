# ADR-0296: Phase 4GK Why Endpoint Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs why` renders each relation edge's source and target endpoint. Users can
filter by relation kind and direction, but still need to scan edges manually
when focusing on a specific endpoint object.

## Decision

`workvcs why` accepts:

- `--source <OBJECT_ID>`
- `--target <OBJECT_ID>`
- `--source-kind <entity|evidence|knowledge_exposure>`
- `--target-kind <entity|evidence|knowledge_exposure>`

The CLI filters the `relation_edges` already returned by the Engine facade.
Endpoint ids are matched against the rendered id for entity, evidence, or
knowledge exposure endpoints.

## Consequences

Users can narrow explanation output to a specific endpoint object or endpoint
kind without changing the `why` engine, schema, or output field names.
