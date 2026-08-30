# ADR-0350: Phase 4IM Merge List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs merge list` renders merge attempts after optional workspace,
target-branch, closed-state, runtime-state, outcome, and limit filters.
Automation needs to assert the resulting merge count directly.

## Decision

`workvcs merge list` accepts optional `--expected-merges COUNT`.

The CLI applies existing filters, renders the same merge list, and appends
`merges_match_expected=true` when the rendered count equals the supplied
expectation. A mismatch returns `QueryInvalid`.

## Consequences

Merge orchestration scripts can fail fast when the filtered merge set is
unexpected. The command does not change merge start, resolve, freeze, continue,
abort, show, list filtering, or replay semantics.
