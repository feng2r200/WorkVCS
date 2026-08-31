# ADR-0395: Phase 4KF Verification Record Expectations CLI

Status: Accepted
Date: 2026-08-31

## Context

`workvcs verification show`, `verification list`, and verification cache
commands already expose script-verifiable expectation arguments. `workvcs
verification record` still requires callers to parse the mutation result and
assert by convention that the recorded Verification targets the intended
object, result, and evidence shape.

Phase 4KF adds post-action expectations to `workvcs verification record` so
acceptance scripts can fail fast after the Engine records the Verification,
without changing Verification semantics or persistence.

## Decision

The CLI adds optional post-action expectations to `workvcs verification record`:

- `--expected-branch BRANCH_ID`
- `--expected-head COMMIT_ID`
- `--expected-target-kind acceptance_criterion|verification_requirement`
- `--expected-target ENTITY_ID`
- `--expected-result passed|failed|inconclusive`
- `--expected-evidence-relations COUNT`

After `Engine::create_verification` returns, the CLI validates supplied
expectations against the returned `VerificationCreateCommit`. Passing checks
append these markers to the normal output:

- `branch_match_expected=true`
- `head_match_expected=true`
- `target_kind_match_expected=true`
- `target_match_expected=true`
- `result_match_expected=true`
- `evidence_relations_match_expected=true`

Mismatches return the existing Verification validation error surface. Generated
identifiers, digests, and timestamps remain output-only values; this slice does
not add expected verification ids, commit ids, changeset ids, version ids,
relation ids, state digests, or timestamp expectations.

## Consequences

Automation can assert the branch, expected input head, verification target,
result, and evidence relation cardinality directly on the mutating command.
The base rendered output stays stable, and callers that do not pass expectation
arguments observe the existing behavior.

This does not change Verification target validation, evidence relation
creation, resource-basis behavior, WorkState commits, Branch HEAD CAS, schema,
Store APIs, or Engine APIs.
