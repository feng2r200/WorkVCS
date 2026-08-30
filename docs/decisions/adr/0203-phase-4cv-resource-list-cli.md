# ADR-0203: Phase 4CV Resource List CLI

Status: Accepted

Date: 2026-08-30

## Context

Phase 4CQ exposed basic Resource CLI operations for create, show, bind, and
workspace association. Resource objects are ObjectIdentity-backed provenance
anchors used by Verification resource basis and observation workflows. The CLI
could inspect a known Resource id but could not enumerate Resources through the
Engine facade.

## Decision

1. Add `ResourceListOptions` and `ResourceListResult` to the core Engine
   facade.
2. Add `Engine::resources` as a read-only Store-boundary API.
3. List Resources through `object_identity` + `resource`, requiring the stored
   object kind to be `resource`.
4. Return full `ResourceSnapshot` values so list output can report binding and
   workspace-association presence without bypassing existing read validation.
5. Sort results by `created_at_us` and `resource_id` for stable output.
6. Add an optional `resource_kind` filter.
7. Add `workvcs resource list STORE [--kind KIND]` to the CLI.
8. Render list output in compact `key=value` form with resource id, kind,
   creation time, binding presence, and workspace-association count.

## Non-Goals

- This slice does not add ResourceObservation listing.
- This slice does not add adapter execution or drift comparison.
- This slice does not change Resource binding or workspace association writes.
- This slice does not make Resource objects part of WorkState.

## Consequences

- CLI users can discover existing Resources before binding, associating, or
  using them as Verification resource basis inputs.
- Resource tooling now has a basic create/show/list loop parallel to Evidence.
- Observation enumeration remains a later, separate tool slice.
