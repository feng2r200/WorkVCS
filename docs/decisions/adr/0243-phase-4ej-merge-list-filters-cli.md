# ADR-0243: Phase 4EJ Merge List Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs merge list` already supports workspace scoping, target branch scoping,
and optional inclusion of closed merge attempts. Operators still need focused
inspection by the runtime state and recorded outcome shown by the existing
merge projection.

The engine already returns merge attempt snapshots with `runtime_state` and
`outcome`. This slice does not change merge lifecycle semantics, conflict
resolution, merge continuation, or storage schema.

## Decision

`workvcs merge list` accepts:

- `--runtime-state <STATE>`
- `--outcome <OUTCOME>`

Both filters are exact CLI-side matches over the existing rendered projection
semantics. Active merge attempts with no outcome match `--outcome none`.

When filters are combined with `--workspace`, `--target-branch`, and
`--include-closed`, all selected conditions must match.

## Consequences

Merge inspection can now target active, completed, aborted, or no-outcome
attempts without adding a new engine query surface.

This slice intentionally does not introduce new merge runtime or outcome
vocabulary validation in the CLI.
