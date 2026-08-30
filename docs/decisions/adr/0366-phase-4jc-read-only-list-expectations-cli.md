# ADR-0366: Phase 4JC Read-Only List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Automation scripts need direct count assertions across remaining read-only CLI
list surfaces. Several list commands already support filtering and limit
options, but callers still had to parse counts externally to fail fast on
unexpected result shapes.

## Decision

The CLI adds the following optional count expectations:

- `workvcs reference list --expected-references COUNT`
- `workvcs checkpoint list --expected-checkpoints COUNT`
- `workvcs bundle import-list --expected-imports COUNT`
- `workvcs verification cache-list --expected-caches COUNT`

Each command applies existing filters and limits first, renders the same list
output, and appends a `*_match_expected=true` marker when the final rendered
count equals the supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

The remaining read-only list surfaces can be used directly in acceptance scripts
without external count parsing. This does not change structural reference
creation, checkpoint creation or validation, bundle import semantics,
verification applicability cache recording, filtering behavior, limit behavior,
or storage schema.
