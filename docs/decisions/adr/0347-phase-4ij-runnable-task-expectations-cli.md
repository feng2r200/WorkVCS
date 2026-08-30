# ADR-0347: Phase 4IJ Runnable Task Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs runnable tasks` renders the current session's runnable task projection.
Automation needs to assert that the projection was evaluated at the expected
branch head and returned the expected number of candidates after filters.

## Decision

`workvcs runnable tasks` accepts optional `--expected-head COMMIT` and
`--expected-candidates COUNT`.

The CLI applies existing task/status/runnable/limit filters, renders the same
projection, and appends `head_matches_expected=true` or
`candidates_match_expected=true` for supplied expectations that match. A
mismatch returns `QueryInvalid`.

## Consequences

Runnable-task automation can fail fast when the projection is stale or when the
filtered candidate count is unexpected. The command does not change runnable
eligibility, dependency readiness, claim coordination, candidate ordering,
claim selection, or task mutation semantics.
