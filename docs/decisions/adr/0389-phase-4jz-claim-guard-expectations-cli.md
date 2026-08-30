# ADR-0389: Phase 4JZ Claim Guard Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs claim guard` evaluates whether a Session may mutate a Task under the
current Claim coordination rules. It reports the boolean decision, guard reason,
and active Claim count. Automation needs to assert those guard outcomes
directly before attempting guarded work.

## Decision

The CLI adds optional expectations to `workvcs claim guard`:

- `--expected-allowed true|false`
- `--expected-reason REASON`
- `--expected-active-claims COUNT`

The command still delegates guard evaluation to `Engine::task_claim_guard`.
Reason expectations use the existing rendered guard reason vocabulary. Passing
checks append `allowed_match_expected=true`, `reason_match_expected=true`, and
`active_claims_match_expected=true`.

## Consequences

Claim coordination checks can now be used directly as script acceptance gates.
This does not change guard evaluation, Claim lifecycle, task mutation rules,
runnable selection, Session state, or storage schema.
