# ADR-0404: Phase 4KO Bundle Smoke Gate

Status: Accepted
Date: 2026-08-31

## Context

ADR-0397 introduced the repository-level CLI smoke workflow, and ADR-0398
through ADR-0403 extended it through verified Task completion, runtime
closeout, Store integrity, Task scheduling, explicit `claim next`, a minimal
Branch fork plus merge lifecycle, and checkpoint create/show/validate/list/latest
coverage.

ADR-0004 defines Store portability as a core V1 boundary: copying a Store
preserves Store identity by default, Bundle import compares same-Store Commit
DAG ancestry and refs, fast-forward is distinguishable from divergence, and
last-write-wins overwrite is forbidden. ADR-0120 through ADR-0125, ADR-0152,
and ADR-0315 through ADR-0381 already accepted and implemented the relevant
local Bundle export, payload directory, validation, preflight, apply, and
expectation commands. The main smoke artifact still did not prove that these
commands work together in an end-to-end portable Store flow.

## Decision

Phase 4KO extends `scripts/smoke-v0.1-cli-workflow.sh` with a Bundle
portability smoke gate using separate temporary Stores.

The smoke workflow creates a source Store, creates a Workspace and initial Task,
copies that Store as an older target, advances the source Store with another
Task, exports the newer source Branch head as a deterministic Bundle directory,
validates the directory with `--require-valid`, preflights the copied target
Store with `--require-can-apply` and same-Store fast-forward expectations,
applies the Bundle with result expectations, and verifies that the target
Store's Branch head, state digest, imported Task status, imported Task priority,
import-show outcome, and import-list count match the applied Bundle.

This is process-level acceptance coverage over existing Bundle APIs. It does
not change Bundle manifest construction, payload layout, import preflight
classification, Bundle apply semantics, Store identity, schema, checkpoint
transport, runtime recovery, divergence handling, external Store import, or
non-task Bundle families.

## Consequences

The main CLI smoke artifact now demonstrates that a copied older local Store
can accept a newer same-Store task-only Bundle through validate, preflight, and
apply without overwriting by policy outside WorkVCS.

Because the Bundle gate uses separate temporary Stores, the primary smoke
Store's final integrity and `doctor` counts remain focused on the main local
workflow and are unchanged by this slice.

Checkpoint transport, divergent Bundle handling, missing Branch creation,
external Store import, typed Acceptance/Verification identity reconstruction,
non-task Bundle apply, and runtime recovery after import remain separate
decisions.
