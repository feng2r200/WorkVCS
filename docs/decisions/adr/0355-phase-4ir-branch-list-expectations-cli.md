# ADR-0355: Phase 4IR Branch List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs branch list` renders Branches for one Workspace after optional name,
lifecycle-state, and limit filters. Automation needs to assert the resulting
Branch count directly.

## Decision

`workvcs branch list` accepts optional `--expected-branches COUNT`.

The CLI applies existing filters, renders the same Branch list, and appends
`branches_match_expected=true` when the rendered count equals the supplied
expectation. A mismatch returns `QueryInvalid`.

## Consequences

Branch bootstrap and fork validation scripts can fail fast when the visible
Branch set is unexpected. The command does not change Branch creation, fork,
head lookup, Branch HEAD movement, or list filtering semantics.
