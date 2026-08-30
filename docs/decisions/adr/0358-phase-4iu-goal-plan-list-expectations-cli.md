# ADR-0358: Phase 4IU Goal Plan List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs goal list` and `workvcs plan list` render upper-level work
organization projections after scoped filters and optional limits. Automation
needs direct count assertions for those rendered result sets.

## Decision

The CLI adds the following optional count expectations:

- `workvcs goal list --expected-goals COUNT`
- `workvcs plan list --expected-plans COUNT`

Each command applies existing filters and limits, renders the same list output,
and appends the corresponding `*_match_expected=true` marker when the rendered
count equals the supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Goal and Plan validation scripts can fail fast when scoped work organization
results differ from the expected shape. These commands do not change Goal/Plan
creation, lifecycle transitions, containment, runnable projection, state
digests, or list filtering behavior.
