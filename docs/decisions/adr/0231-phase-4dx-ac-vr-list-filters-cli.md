# ADR-0231: Phase 4DX AC And VR List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

Acceptance Criterion and Verification Requirement list commands can enumerate
semantic state at a Branch head or historical Commit, but the read surface is
too broad once a task has multiple criteria or criteria have multiple
requirements. The existing output already exposes the stable fields needed for
targeted inspection.

The first AC/VR list slice intentionally kept filtering out of scope. This
slice adds only read-side filtering and does not change semantic mutation,
effective-status projection, storage schema, or verification execution.

## Decision

1. Add `--task TASK_ENTITY_ID` and `--classification CLASSIFICATION` to
   `workvcs ac list`.
2. Add `--criterion ACCEPTANCE_CRITERION_ENTITY_ID` and `--local-key KEY` to
   `workvcs vr list`.
3. Resolve the existing Branch/Commit target exactly as before.
4. Load snapshots through the existing Engine facade and filter before
   rendering.
5. Reuse the existing CLI acceptance-criterion classification vocabulary:
   `required` and `optional`.

## Non-Goals

- This slice does not add new AC or VR lifecycle states.
- This slice does not change Acceptance Criterion effective-status semantics.
- This slice does not add text search, statement matching, or pagination.
- This slice does not add Verification execution or resource matching.
- This slice does not change SQLite schema, replay, or core snapshot APIs.

## Consequences

- CLI users can inspect focused AC/VR subsets without post-processing full
  lists.
- The tool surface remains a thin read-only wrapper over existing Engine
  snapshots.
- Future filtering can continue this pattern when the underlying semantic field
  is already part of the confirmed snapshot surface.
