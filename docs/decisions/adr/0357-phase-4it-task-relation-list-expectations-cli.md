# ADR-0357: Phase 4IT Task Relation List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Task graph tooling relies on two read-only relation list surfaces:
`workvcs task scheduling-list` and `workvcs task containment-list`. Both render
`relations` after scoped filters and optional limits. Automation needs to
assert that relation count directly.

## Decision

Both commands accept optional `--expected-relations COUNT`.

The CLI applies existing filters, renders the same list output, and appends
`relations_match_expected=true` when the rendered count equals the supplied
expectation. A mismatch returns `QueryInvalid`.

## Consequences

Task graph validation scripts can fail fast when dependency, ordering, or
containment relation sets are unexpected. These commands do not change Task
mutation, scheduling relation creation, containment creation, graph invariants,
or list filtering behavior.
