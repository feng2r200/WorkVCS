# ADR-0239: Phase 4EF Branch List Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

Phase 4 requires the local CLI to expose narrower read-side inspection tools over
already implemented engine projections. `workvcs branch list` was scoped by
workspace, but it did not allow the caller to select a specific branch name or
lifecycle state.

The existing engine API already returns authoritative branch heads for one
workspace. This slice does not require a new storage query, schema migration,
branch lifecycle transition, or branch domain model change.

## Decision

`workvcs branch list` accepts:

- `--name <NAME>`
- `--lifecycle-state <STATE>`

Both filters are exact, case-sensitive CLI-side filters over the current
`BranchHead` projection returned by `Engine::list_branches`.

When both filters are present, the result must satisfy both conditions.

## Consequences

Branch inspection can now be narrowed without broadening the storage or engine
surface.

The CLI intentionally does not validate lifecycle-state vocabulary in this
slice. It matches the stored projection text exactly and returns an empty list
when no branch matches.
