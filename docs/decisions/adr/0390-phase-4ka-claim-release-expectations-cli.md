# ADR-0390: Phase 4KA Claim Release Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs claim release` ends an active Claim for a Session and reports the
released lifecycle state. Automation needs to verify the release result without
following up with a separate `claim show` command.

## Decision

The CLI adds optional post-release expectations to `workvcs claim release`:

- `--expected-session SESSION_ID`
- `--expected-lifecycle-state STATE`

The command still delegates to `Engine::release_claim`. Passing checks append
`session_match_expected=true` and `lifecycle_state_match_expected=true`.
Mismatches use the existing Claim validation error surface.

## Consequences

Claim release can now be used directly as a script-verifiable coordination
step. Expectations are evaluated after release; mismatch handling does not
change Claim lifecycle semantics, Session state, runnable selection, or storage
schema.
