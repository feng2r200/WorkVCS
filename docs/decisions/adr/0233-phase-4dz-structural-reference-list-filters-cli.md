# ADR-0233: Phase 4DZ Structural Reference List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

Structural Reference list can enumerate reference relations at a Branch head or
historical Commit, but users need to inspect relations by endpoint identity and
endpoint kind. The existing list output already exposes referrer entity,
target entity, referrer kind, and target kind.

This slice continues the read-side CLI filter pattern. It narrows rendered
snapshots without changing structural reference creation, relation semantics,
storage schema, replay, or the Engine snapshot API.

## Decision

1. Add `--referrer ENTITY_ID` to `workvcs reference list`.
2. Add `--target ENTITY_ID` to `workvcs reference list`.
3. Add `--referrer-kind KIND` to `workvcs reference list`.
4. Add `--target-kind KIND` to `workvcs reference list`.
5. Resolve the existing Branch/Commit target exactly as before.
6. Load Structural Reference snapshots through the existing Engine facade and
   apply all supplied filters before rendering.
7. Keep the endpoint-kind vocabulary limited to the confirmed structural
   reference endpoint kinds: `goal`, `plan`, and `task`.

## Non-Goals

- This slice does not add new structural reference endpoint kinds.
- This slice does not add relation deletion, lifecycle state, or pagination.
- This slice does not add graph traversal or why-neighborhood expansion.
- This slice does not change SQLite schema, replay, or core snapshot APIs.

## Consequences

- CLI users can inspect focused structural reference subsets without
  post-processing full relation lists.
- Structural reference review remains a thin read-only wrapper over existing
  Engine snapshots.
- Future endpoint filters can follow this pattern only when the endpoint field
  is already part of the confirmed snapshot surface.
