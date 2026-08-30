# ADR-0241: Phase 4EH Resource List State Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs resource list` exposed resource kind filtering, but operators also
need to quickly find resources by binding state or workspace association while
inspecting local tool state.

The existing engine resource list projection already includes binding presence
and workspace association snapshots. No new storage query or schema change is
required.

## Decision

`workvcs resource list` accepts:

- `--bound <true|false>`
- `--workspace <WORKSPACE_ID>`

Both are CLI-side filters over the `ResourceSnapshot` values returned by the
existing engine list call. `--workspace` parses the UUID canonically and matches
resources that have an association with that workspace id.

When combined with `--kind`, all filters must match.

## Consequences

Resource inspection can now answer common local operator questions without
expanding the engine API.

This slice intentionally does not validate workspace existence for `--workspace`
on `resource list`; a well-formed unmatched id simply returns an empty list.
