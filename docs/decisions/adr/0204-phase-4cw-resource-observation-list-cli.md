# ADR-0204: Phase 4CW Resource Observation List CLI

Status: Accepted

Date: 2026-08-30

## Context

Resource observations capture adapter-produced fingerprints and summaries used
by Verification resource basis and applicability cache workflows. The CLI
already supported recording an observation and reading a known observation id,
but it could not enumerate observations through the Engine facade.

## Decision

1. Add `ResourceObservationListOptions` and `ResourceObservationListResult` to
   the core Engine facade.
2. Add `Engine::resource_observations` as a read-only Store-boundary API.
3. List observations through `object_identity` + `resource_observation`,
   requiring the stored object kind to be `resource_observation`.
4. Return full `ResourceObservationSnapshot` values so list output reuses the
   same validation and canonical summary parsing as `observation-show`.
5. Sort results by `captured_at_us` and `observation_id` for stable output.
6. Support optional filtering by `resource_id` and `adapter_kind`.
7. Add `workvcs resource observation-list STORE [--resource RESOURCE]
   [--adapter-kind KIND]` to the CLI.
8. Render compact `key=value` rows with observation id, resource id, adapter
   fields, captured time, fingerprint, summary JSON, detail presence, and source
   session id.

## Non-Goals

- This slice does not execute adapters.
- This slice does not compare resource drift.
- This slice does not add Observation retention or storage-location behavior.
- This slice does not change Verification resource basis semantics.

## Consequences

- CLI users can discover resource observations before recording or explaining
  Verification resource basis.
- The Resource tool surface now supports create/show/list and
  observe/show/list loops through the Engine facade.
- Adapter execution and drift workflows remain later slices.
