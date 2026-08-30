# ADR-0235: Phase 4EB Task Scheduling List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

Task scheduling list can enumerate scheduling relations at a Branch head or
historical Commit, but users need to inspect relations by type and task
endpoint. The existing list output already exposes relation type, source task,
and target task.

This slice continues the read-side CLI filter pattern. It narrows rendered
snapshots without changing scheduling relation creation, dependency semantics,
runnable-task projection, storage schema, replay, or the Engine snapshot API.

## Decision

1. Add `--relation-type TYPE` to `workvcs task scheduling-list`.
2. Add `--source-task TASK_ENTITY_ID` to `workvcs task scheduling-list`.
3. Add `--target-task TASK_ENTITY_ID` to `workvcs task scheduling-list`.
4. Resolve the existing Branch/Commit target exactly as before.
5. Load Task Scheduling Relation snapshots through the existing Engine facade
   and apply all supplied filters before rendering.
6. Keep the relation-type vocabulary limited to the confirmed scheduling
   relation types: `depends_on` and `ordered_before`.

## Non-Goals

- This slice does not add new scheduling relation types.
- This slice does not change dependency readiness or runnable-task projection.
- This slice does not add graph traversal, transitive dependency expansion, or
  pagination.
- This slice does not change SQLite schema, replay, or core snapshot APIs.

## Consequences

- CLI users can inspect focused scheduling relation subsets without
  post-processing full relation lists.
- Dependency and manual ordering review can target one relation type or one
  endpoint directly.
- The command remains a thin read-only wrapper over existing Engine snapshots.
