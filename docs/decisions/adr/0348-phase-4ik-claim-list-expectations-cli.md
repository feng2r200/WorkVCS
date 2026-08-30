# ADR-0348: Phase 4IK Claim List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs claim list` renders active claims for a session after optional task,
mode, and limit filters. Automation needs to assert the resulting claim count
without parsing and comparing it externally.

## Decision

`workvcs claim list` accepts optional `--expected-claims COUNT`.

The CLI applies existing list filters, renders the same claim list, and appends
`claims_match_expected=true` when the rendered claim count equals the supplied
expectation. A mismatch returns `QueryInvalid`.

## Consequences

Claim coordination scripts can fail fast when a session has an unexpected
number of active claims. The command does not change claim creation, release,
guard evaluation, next-task selection, or list filtering semantics.
