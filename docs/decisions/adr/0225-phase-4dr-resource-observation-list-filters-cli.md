# ADR-0225: Phase 4DR Resource Observation List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

ResourceObservation already records `resource_id`, `adapter_kind`,
`adapter_schema_version`, and optional `source_session_id`. The CLI list command
could filter by resource and adapter kind, but provenance-oriented workflows
also need to recover observations captured by one Session or one adapter schema
revision without scanning all observation rows.

ResourceObservation remains immutable Store-level provenance. It is not a
semantic WorkState object.

## Decision

1. Extend `ResourceObservationListOptions` with optional
   `adapter_schema_version` and `source_session_id` filters.
2. Preserve existing `resource_id` and `adapter_kind` filters and allow all
   filters to combine.
3. Add `--adapter-schema-version VERSION` and `--source-session SESSION` to
   `workvcs resource observation-list`.
4. Validate adapter schema versions with the existing positive-version rule.
5. Parse `SESSION` as a typed `SessionId`.
6. Keep list rendering unchanged, because entries already expose the filtered
   fields.

## Non-Goals

- This slice does not execute adapters.
- This slice does not compare resource drift.
- This slice does not change ResourceObservation creation semantics.
- This slice does not add commit-scoped resource observation projections.

## Consequences

- CLI users can query ResourceObservation provenance by resource, adapter kind,
  adapter schema revision, and source Session.
- Verification resource-basis audits can identify the exact observation family
  used by a run without lower-level storage inspection.
- The change remains inside the existing Engine facade and Store provenance
  boundary.
