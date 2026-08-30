# ADR-0345: Phase 4IH Claim Show Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs claim show` exposes a claim's current lifecycle state and claim mode.
Automation needs to assert those fields directly before allowing guarded task
mutation or confirming that a claim was released.

## Decision

`workvcs claim show` accepts optional `--expected-lifecycle-state VALUE` and
`--expected-mode VALUE`.

The CLI reads the claim through the Engine facade and validates expectation
values against the existing claim vocabulary. A matching lifecycle state returns
`lifecycle_state_matches_expected=true`; a matching mode returns
`mode_matches_expected=true`. A mismatch returns `ClaimInvalid`.

## Consequences

Claim coordination scripts can fail fast when a claim is in an unexpected state
or mode. The command does not change claim creation, next-task selection,
release, list filtering, guard evaluation, or task mutation semantics.
