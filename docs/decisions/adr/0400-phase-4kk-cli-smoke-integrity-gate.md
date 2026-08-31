# ADR-0400: Phase 4KK CLI Smoke Integrity Gate

Status: Accepted
Date: 2026-08-31

## Context

ADR-0397 introduced a repository-level CLI smoke workflow, ADR-0398 extended it
through verified Task completion, and ADR-0399 closed the runtime coordination
state by releasing the selected Claim and ending the active Session.

The workflow now demonstrates the user-visible local CLI loop, but it does not
yet end with the Engine-owned integrity gates that local operators should run
before trusting a generated Store. ADR-0369 added `--require-valid` to
`workvcs store integrity` and `workvcs doctor`; ADR-0382 and ADR-0383 added
count expectations for both commands. Those existing command surfaces can make
the smoke workflow prove that the completed Store is still structurally valid
without direct SQL or new integrity semantics.

## Decision

Phase 4KK extends `scripts/smoke-v0.1-cli-workflow.sh` with a final integrity
gate after runtime closeout.

The smoke workflow now proves:

- `workvcs store integrity --require-valid` succeeds after the full CLI smoke
  workflow;
- `store integrity` reports expected counts for checked Branches, Commits,
  ChangeSets, ChangeOperations, ChangeSet causal anchors, Events, Checkpoints,
  and invalid Checkpoints;
- `workvcs doctor --require-valid` succeeds on the same completed Store;
- `doctor` reports the same expected integrity counts through its combined
  manifest-plus-integrity entrypoint.

This slice does not change Engine behavior, Store behavior, schema, integrity
algorithms, checkpoint validation, runtime semantics, or CLI command behavior.
It only makes the existing integrity and doctor gates part of the
process-level smoke workflow.

## Consequences

The repository smoke workflow now finishes with an explicit Store health gate.
Future smoke extensions should keep integrity counts tied to the workflow they
actually run; if a future semantic step changes the number of generated commits,
operations, events, or checkpoints, that slice should update the expectations
with fresh process-boundary evidence instead of treating the counts as a
separate specification.
