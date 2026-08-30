# ADR-0365: Phase 4JB Knowledge Space Derived Exposure Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Knowledge Space workflows expose read-only derived exposure query surfaces for
available, source-stale, and historical exposures. These commands already apply
scoped filters and optional limits, but automation needs direct count assertions
on the rendered result sets.

`knowledge-space-refresh-source-statuses` is intentionally excluded from this
slice because it mutates source-status state; a post-refresh expectation
mismatch would report failure after changing store state.

## Decision

The CLI adds the following optional count expectations:

- `workvcs store knowledge-space-available-exposures --expected-exposures COUNT`
- `workvcs store knowledge-space-source-stale-exposures --expected-exposures COUNT`
- `workvcs store knowledge-space-historical-exposures --expected-exposures COUNT`

Each command applies existing filters and limits, renders the same list output,
and appends `exposures_match_expected=true` when the rendered count equals the
supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Knowledge Space exposure validation scripts can fail fast when scoped derived
exposure query results differ from the expected shape. These commands do not
change Knowledge Exposure creation, lifecycle transitions, source-status
refresh, adoption, source-stale classification, historical classification,
digest semantics, or list filtering behavior.
