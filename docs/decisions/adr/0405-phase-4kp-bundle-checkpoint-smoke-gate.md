# ADR-0405: Phase 4KP Bundle Checkpoint Smoke Gate

Status: Accepted
Date: 2026-08-31

## Context

ADR-0404 added repository-level smoke coverage for a same-Store task-only
Bundle fast-forward flow. That gate proved export, deterministic directory
validation, preflight, apply, target Branch head movement, imported Task
readback, and import journal readback. It intentionally kept the exported
Bundle free of checkpoint candidates.

ADR-0159 already accepted and implemented same-Store Bundle apply support for
checkpoint candidate metadata and checkpoint status rows. The CLI also already
renders `imported_checkpoints` and `imported_checkpoint_statuses` from
`bundle apply-dir`, but scripts cannot yet assert those counts directly, and
the repository-level smoke workflow does not prove that a checkpoint attached
to an exported commit survives Bundle transport into a copied older Store.

## Decision

Phase 4KP extends `workvcs bundle apply-dir` with optional result expectations:

- `--expected-imported-checkpoints COUNT`
- `--expected-imported-checkpoint-statuses COUNT`

The command still applies through the existing Engine path. Passing checks
append `imported_checkpoints_match_expected=true` or
`imported_checkpoint_statuses_match_expected=true`. A mismatch returns
`QueryInvalid`.

Phase 4KP also extends the repository CLI smoke workflow's separate Bundle
portability gate. After the source Store advances to the exported head, the
smoke workflow creates a checkpoint for that head, exports a Bundle that
contains one checkpoint candidate, validates and preflights the directory,
applies it to the copied target Store with checkpoint count expectations, and
verifies target `checkpoint latest`, `checkpoint show`, and `checkpoint
validate --require-valid`.

This is CLI expectation and process-level acceptance coverage over existing
Bundle checkpoint APIs. It does not change Bundle manifest construction,
payload layout, checkpoint creation, checkpoint validation, checkpoint
authority, checkpoint raw payload storage, checkpoint restore, checkpoint
scheduling, checkpoint eviction, Store identity, schema, divergence handling,
external Store import, non-task Bundle families, or runtime recovery.

## Consequences

Acceptance scripts can now fail fast when a Bundle apply unexpectedly omits or
duplicates checkpoint metadata/status restoration.

The main smoke artifact now demonstrates that a copied older local Store can
accept a newer same-Store Bundle whose target commit has rebuildable checkpoint
metadata, and that the imported checkpoint remains validated by replay rather
than becoming a second history authority.

Checkpoint restore, checkpoint scheduling, checkpoint eviction, raw checkpoint
payload storage, divergent Bundle handling, missing Branch creation, external
Store import, and runtime recovery after import remain separate decisions.
