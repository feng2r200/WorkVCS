# ADR-0234: Phase 4EA Primary Containment List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

Primary containment list can enumerate containment relations at a Branch head or
historical Commit, but users need to inspect relations by parent or child
endpoint. The existing list output already exposes parent entity, child entity,
parent kind, and child kind.

This slice follows the established read-side CLI filter pattern. It narrows
rendered snapshots without changing containment creation, scheduling,
runnable-task projection, storage schema, replay, or the Engine snapshot API.

## Decision

1. Add `--parent ENTITY_ID` to `workvcs task containment-list`.
2. Add `--child ENTITY_ID` to `workvcs task containment-list`.
3. Add `--parent-kind KIND` to `workvcs task containment-list`.
4. Add `--child-kind KIND` to `workvcs task containment-list`.
5. Resolve the existing Branch/Commit target exactly as before.
6. Load Primary Containment snapshots through the existing Engine facade and
   apply all supplied filters before rendering.
7. Keep the endpoint-kind vocabulary limited to the confirmed containment
   endpoint kinds: `goal`, `plan`, and `task`.

## Non-Goals

- This slice does not add new containment endpoint kinds.
- This slice does not change containment graph validation or runnable-task
  projection.
- This slice does not add graph traversal, descendant expansion, or pagination.
- This slice does not change SQLite schema, replay, or core snapshot APIs.

## Consequences

- CLI users can inspect focused primary containment subsets without
  post-processing full relation lists.
- Runnable and hierarchy workflows can verify a specific parent or child
  surface directly.
- The command remains a thin read-only wrapper over existing Engine snapshots.
