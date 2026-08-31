# ADR-0401: Phase 4KL CLI Smoke Scheduling Claim-Next Gate

Status: Accepted
Date: 2026-08-31

## Context

ADR-0397 introduced the repository-level CLI smoke workflow, ADR-0398 extended
it through verified Task completion, ADR-0399 closed runtime state, and
ADR-0400 added final Store integrity and doctor gates.

That workflow still uses a single Task until selection time. It therefore does
not show the process-level behavior of Task scheduling relations or the
lower-level `workvcs claim next` command even though those surfaces are already
implemented and script-verifiable. ADR-0022 defines `depends_on` and
`ordered_before` as independent Task scheduling relations. ADR-0023 defines
dependency readiness. ADR-0176 defines explicit manual order for runnable
projection. ADR-0048 and ADR-0386 define `claim next` and its expectation flags.

Priority remains a separate Task field, but the integer priority direction is
not frozen. The smoke workflow can therefore assert priority persistence and
rendering, but it must not assert that higher or lower integers select first.

## Decision

Phase 4KL extends `scripts/smoke-v0.1-cli-workflow.sh` with a scheduling and
explicit claim-next gate inside the existing local CLI workflow.

The smoke workflow now proves:

- Task creation can persist and report caller-provided priority values;
- a dependent Task can be related to a prerequisite Task through `depends_on`;
- the same two Tasks can also carry an independent `ordered_before` relation;
- `task scheduling-list` reports the expected relation count and endpoints;
- runnable projection reports the dependent Task as dependency-blocked while
  its prerequisite is still pending;
- `workvcs claim next` selects and claims the currently runnable prerequisite
  Task with explicit post-selection expectations;
- after the prerequisite is completed and its claim is released, the earlier
  acceptance-criterion Verification is observed as stale and a fresh
  Verification is recorded at the updated Branch head;
- after the Session focus is cleared and the acceptance criterion is verified
  again, `workvcs next` selects the dependent Task through the combined
  operator workflow.

This slice does not change Engine behavior, Store behavior, schema, runnable
ordering semantics, priority semantics, Claim coordination, Session semantics,
integrity algorithms, or CLI command behavior. It only makes existing
scheduling and claim-next capabilities part of the process-level smoke
workflow.

## Consequences

The repository smoke workflow now demonstrates a two-Task local scheduling loop
instead of only a single-Task completion path. Future smoke extensions should
keep integrity counts tied to the workflow they actually run; adding or
removing Task, relation, Claim, Session, Verification, or completion steps must
update the final count expectations with fresh process-boundary evidence.

The unresolved Task priority direction remains out of scope. A later accepted
decision is required before priority values can become an authoritative
selection-order assertion.
