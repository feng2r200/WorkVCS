# ADR-0360: Phase 4IW Resource List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Resource workflows expose three read-only list surfaces: `resource list`,
`resource workspace-association-list`, and `resource observation-list`. Each
surface already applies scoped filters and optional limits, but automation
needs direct count assertions on the rendered result sets.

## Decision

The CLI adds the following optional count expectations:

- `workvcs resource list --expected-resources COUNT`
- `workvcs resource workspace-association-list --expected-workspace-associations COUNT`
- `workvcs resource observation-list --expected-observations COUNT`

Each command applies existing filters and limits, renders the same list output,
and appends the corresponding `*_match_expected=true` marker when the rendered
count equals the supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Resource automation can fail fast when scoped resources, workspace-resource
associations, or resource observations differ from the expected shape. These
commands do not change Resource creation, binding, workspace association,
observation recording, content digest semantics, or list filtering behavior.
