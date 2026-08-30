# ADR-0361: Phase 4IX Record List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Record workflows expose read-only list surfaces for records, record-to-record
relations, and record-to-knowledge relations. These commands already apply
scoped filters and optional limits, but automation needs direct count
assertions on the rendered result sets.

## Decision

The CLI adds the following optional count expectations:

- `workvcs record list --expected-records COUNT`
- `workvcs record relation-list --expected-relations COUNT`
- `workvcs record knowledge-relation-list --expected-relations COUNT`

Each command applies existing filters and limits, renders the same list output,
and appends the corresponding `*_match_expected=true` marker when the rendered
count equals the supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Record validation scripts can fail fast when scoped record or relation results
differ from the expected shape. These commands do not change Record creation,
lifecycles, relation creation/removal/restore, state digests, WorkState
mapping, or list filtering behavior.
