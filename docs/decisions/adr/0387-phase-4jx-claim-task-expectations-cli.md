# ADR-0387: Phase 4JX Claim Task Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs claim task` creates a Claim for an explicitly selected Task in the
current Session context. Operators and automation need to verify the created
Claim without issuing a separate `claim show` command.

## Decision

The CLI adds optional post-claim expectations to `workvcs claim task`:

- `--expected-workspace WORKSPACE_ID`
- `--expected-branch BRANCH_ID`
- `--expected-task TASK_ENTITY_ID`
- `--expected-mode exclusive|shared`
- `--expected-lifecycle-state STATE`

The command still delegates Claim creation to `Engine::claim_task`. Passing
checks append field-specific `*_match_expected=true` markers. Identifier
mismatches return `QueryInvalid`; mode and lifecycle vocabulary mismatches use
the existing claim validation paths.

## Consequences

Explicit Task claiming can now be used directly as a script-verifiable
coordination step. Expectations are evaluated after Claim creation; a mismatch
reports failed acceptance for the observed Claim rather than changing mutation
rollback semantics. This does not change claim coordination, runnable
eligibility, Session state, or storage schema.
