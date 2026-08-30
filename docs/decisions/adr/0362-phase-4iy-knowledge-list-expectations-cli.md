# ADR-0362: Phase 4IY Knowledge List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Knowledge workflows expose read-only list surfaces for Knowledge entities and
Knowledge supersession relations. These commands already apply scoped filters
and optional limits, but automation needs direct count assertions on the
rendered result sets.

## Decision

The CLI adds the following optional count expectations:

- `workvcs knowledge list --expected-knowledge COUNT`
- `workvcs knowledge relation-list --expected-relations COUNT`

Each command applies existing filters and limits, renders the same list output,
and appends the corresponding `*_match_expected=true` marker when the rendered
count equals the supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Knowledge validation scripts can fail fast when scoped Knowledge or supersession
relation results differ from the expected shape. These commands do not change
Knowledge creation, lifecycle transitions, supersession relation creation or
restore/remove behavior, state digests, WorkState mapping, or list filtering.
