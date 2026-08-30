# ADR-0367: Phase 4JD WorkState Diff Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs diff` is a common acceptance-script surface for checking whether a
semantic operation changed the WorkState in the intended shape. Existing diff
filters and limit handling expose the final entity and relation change counts,
but callers still had to parse those counts externally.

## Decision

The CLI adds two optional expectations to `workvcs diff`:

- `--expected-entity-changes COUNT`
- `--expected-relation-changes COUNT`

The command applies existing target, change-kind, entity/relation id, and limit
filters first. It then renders the same diff output and appends
`entity_changes_match_expected=true` or
`relation_changes_match_expected=true` when the final rendered count equals the
supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Diff-based acceptance scripts can fail fast without external count parsing.
This does not change WorkState diff semantics, filtering behavior, limit
behavior, replay, or storage schema.
