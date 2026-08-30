# ADR-0376: Phase 4JM Bundle Apply Result Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle apply-dir` reports whether a bundle was applied, the final
outcome, imported Commit and EntityVersion counts, and updated Branch head
count. Scripts need to assert those stable result fields after an apply attempt.

## Decision

The CLI adds optional result expectations to `workvcs bundle apply-dir`:

- `--expected-outcome OUTCOME`
- `--expected-imported-commits COUNT`
- `--expected-imported-entity-versions COUNT`
- `--expected-updated-branch-heads COUNT`

The command still applies through the existing Engine path. Passing checks
append `outcome_matches_expected=true` or the corresponding
`*_match_expected=true` count marker. A mismatch returns `QueryInvalid`.

## Consequences

Bundle apply scripts can verify the applied outcome and core import counts from
the command result. Because apply is a mutating operation, these expectations
are post-apply checks; callers that need failure without mutation must run
`bundle preflight-dir` first. This does not change bundle apply semantics,
branch head movement, import attempt recording, or storage schema.
