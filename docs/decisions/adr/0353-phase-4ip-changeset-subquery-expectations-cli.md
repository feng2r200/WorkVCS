# ADR-0353: Phase 4IP Changeset Subquery Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs changeset operations` and `workvcs changeset anchors` render
changeset-local subqueries after optional filters and limits. Automation needs
to assert those result counts directly.

## Decision

`workvcs changeset operations` accepts optional `--expected-operations COUNT`.
`workvcs changeset anchors` accepts optional `--expected-anchors COUNT`.

The CLI applies existing filters, renders the same subquery output, and appends
`operations_match_expected=true` or `anchors_match_expected=true` when the
rendered count equals the supplied expectation. A mismatch returns
`QueryInvalid`.

## Consequences

Changeset validation scripts can fail fast when the operation or causal-anchor
set is unexpected. These commands do not change changeset creation, commit
linkage, event recording, causal-anchor semantics, or list filtering.
