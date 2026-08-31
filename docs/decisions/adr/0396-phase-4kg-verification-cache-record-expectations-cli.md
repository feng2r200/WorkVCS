# ADR-0396: Phase 4KG Verification Cache Record Expectations CLI

Status: Accepted
Date: 2026-08-31

## Context

`workvcs verification cache-show` and `verification cache-list` already expose
script-verifiable expectation arguments for branch-scoped Verification
Applicability Cache readback. `workvcs verification cache-record` is the
mutating command that records externally observed Resource stamps and receives
the Engine-computed applicability result, but callers currently need a second
read command to assert the cache write outcome.

Phase 4KG adds post-action expectations to `workvcs verification cache-record`
so resource-backed Verification workflows can fail fast at the cache write
boundary without changing applicability computation or cache persistence.

## Decision

The CLI adds optional post-action expectations to
`workvcs verification cache-record`:

- `--expected-evaluated-commit COMMIT_ID`
- `--expected-applicability applicable|stale|unknown`
- `--expected-reason-code REASON_CODE`
- `--expected-resource-stamps COUNT`

After `Engine::record_verification_applicability` returns, the CLI validates
supplied expectations against the returned
`VerificationApplicabilityCacheSnapshot`. Passing checks append these markers
to the normal output:

- `evaluated_commit_matches_expected=true`
- `applicability_matches_expected=true`
- `reason_code_matches_expected=true`
- `resource_stamps_match_expected=true`

Mismatches return the existing Verification cache validation error surface.
The command continues to accept caller-supplied Resource stamps and lets Core
compute applicability and reason code.

## Consequences

Resource-backed Verification smoke workflows can assert the cache write result
directly on the mutating command. Callers that do not pass expectation
arguments observe the existing rendered output.

This does not change Resource Basis validation, Resource stamp validation,
applicability computation, Acceptance Criterion status projection, WorkState
commits, Events, schema, Store APIs, or Engine APIs.
