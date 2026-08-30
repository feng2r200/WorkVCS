# ADR-0343: Phase 4IF Verification Cache Show Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs verification cache-show` renders the cached applicability result for a
Verification on a branch. Automation needs to assert the evaluated commit,
applicability, and reason code directly at lookup time.

## Decision

`workvcs verification cache-show` accepts optional
`--expected-evaluated-commit COMMIT_ID`, `--expected-applicability VALUE`, and
`--expected-reason-code VALUE`.

The CLI reads the cache through the Engine facade and returns
`evaluated_commit_matches_expected=true`,
`applicability_matches_expected=true`, or
`reason_code_matches_expected=true` for each supplied matching expectation. A
missing cache or mismatch returns `QueryInvalid`.

## Consequences

Verification effective-status scripts can fail fast when an applicability cache
is missing, stale, or recorded against an unexpected commit or reason. The
command does not change cache recording, list filtering, Verification state, or
resource observation semantics.
