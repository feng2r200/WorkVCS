# ADR-0364: Phase 4JA Knowledge Space Exposure List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Knowledge Space and Knowledge Exposure workflows expose direct read-only list
surfaces. These commands already apply scoped filters and optional limits, but
automation needs direct count assertions on the rendered result sets.

## Decision

The CLI adds the following optional count expectations:

- `workvcs store knowledge-space-list --expected-knowledge-spaces COUNT`
- `workvcs store knowledge-exposure-list --expected-exposures COUNT`

Each command applies existing filters and limits, renders the same list output,
and appends the corresponding `*_match_expected=true` marker when the rendered
count equals the supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Knowledge Space and Knowledge Exposure validation scripts can fail fast when
scoped results differ from the expected shape. These commands do not change
Knowledge Space creation, Knowledge Exposure lifecycle transitions, source
status handling, digest semantics, or list filtering behavior.
