# ADR-0403: Phase 4KN Checkpoint Smoke Gate

Status: Accepted
Date: 2026-08-31

## Context

ADR-0397 introduced the repository-level CLI smoke workflow, and ADR-0398
through ADR-0402 extended it through verified Task completion, runtime
closeout, Store integrity, Task scheduling, explicit `claim next`, and a
minimal Branch fork plus merge lifecycle.

ADR-0115 through ADR-0118 already accepted and implemented checkpoint create,
show, validate, list, and latest-usable selection. The smoke workflow still
ended with zero checkpoints, so it did not prove the V0.1 CLI can exercise the
checkpoint metadata path in the same end-to-end artifact that validates history,
runtime, merge, and integrity behavior.

## Decision

Phase 4KN extends `scripts/smoke-v0.1-cli-workflow.sh` with a checkpoint smoke
gate over the live smoke Branch head.

The smoke workflow creates a checkpoint for the current head commit, captures
the checkpoint id and content digest from CLI output, verifies `checkpoint
show` with expected state and content digests, runs `checkpoint validate
--require-valid`, verifies `checkpoint list` for the commit, and verifies
`checkpoint latest --require-found --expected-checkpoint`.

The final Store integrity and `doctor` checks now require one checked
checkpoint and zero invalid checkpoints.

This is process-level acceptance coverage over the existing checkpoint APIs.
It does not change checkpoint payload construction, replay authority,
checkpoint selection, checkpoint status mutation semantics, restore behavior,
scheduling, eviction, raw payload persistence, schema, Bundle export/import,
or Store lineage.

## Consequences

The main CLI smoke artifact now demonstrates that a local WorkVCS Store can
materialize and validate a rebuildable checkpoint for the same Work-State
history it later checks through integrity and `doctor`.

Checkpoint restore, checkpoint scheduling, checkpoint eviction, persisted raw
checkpoint payload bytes, and checkpoint transport through Bundle workflows
remain separate decisions.
