# ADR-0344: Phase 4IG Merge Show Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs merge show` renders the runtime state and outcome for a merge attempt.
Automation needs to assert these status fields directly before deciding whether
to resolve, continue, abort, or inspect a completed merge.

## Decision

`workvcs merge show` accepts optional `--expected-runtime-state VALUE` and
`--expected-outcome VALUE`.

The CLI reads the merge attempt through the Engine facade and returns
`runtime_state_matches_expected=true` or `outcome_matches_expected=true` for
each supplied matching expectation. A mismatch returns `QueryInvalid`.

## Consequences

Merge orchestration scripts can fail fast when a merge attempt is in an
unexpected runtime state or has an unexpected outcome. The command does not
change merge start, resolution, freeze, continue, abort, list filtering, or
replay semantics.
