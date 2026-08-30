# ADR-0386: Phase 4JW Claim Next Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs claim next` is the mutating command that selects runnable work for a
Session and creates a Claim when a candidate is available. Automation needs to
verify the selected result without issuing a separate read command.

## Decision

The CLI adds optional post-selection expectations to `workvcs claim next`:

- `--expected-selected true|false`
- `--expected-head COMMIT_ID`
- `--expected-inspected-candidates COUNT`
- `--expected-task TASK_ENTITY_ID`
- `--expected-mode exclusive|shared`
- `--expected-lifecycle-state STATE`

The command still delegates selection and Claim creation to
`Engine::claim_next_task`. Passing checks append field-specific
`*_match_expected=true` markers. Mismatches return `QueryInvalid` or the
existing claim validation error for claim state and mode vocabulary checks.

## Consequences

Task acquisition can now be used directly as a script-verifiable work selection
step. Expectations are evaluated after selection; a mismatch reports failed
acceptance for the observed result rather than changing Claim creation rollback
semantics. This does not change runnable ordering, dependency readiness, Claim
coordination, Session state, or storage schema.
