# ADR-0227: Phase 4DT Store Migration List Filters CLI

Status: Accepted

Date: 2026-08-30

## Context

Store migration attempts are immutable infrastructure provenance. The CLI can
record, show, and list migration attempts, but `migration-list` only supported a
limit. Operators auditing Store changes often need to narrow the journal by
the migration tool version or by the recorded outcome before inspecting detail.

This remains an audit query over existing migration rows. It does not execute
or alter migrations.

## Decision

1. Extend `StoreMigrationListOptions` with optional `tool_version` and
   `outcome` filters.
2. Preserve the existing limit behavior and apply the limit after filters.
3. Add `--tool-version VERSION` and `--outcome OUTCOME` to
   `workvcs store migration-list`.
4. Reuse the existing stored-text validation rules for both filter values.
5. Keep list rendering unchanged, because entries already expose tool version
   and outcome.

## Non-Goals

- This slice does not perform a Store migration.
- This slice does not mutate StoreManifest or schema state.
- This slice does not add rollback, retry, or repair semantics.
- This slice does not change Store lineage or Bundle import behavior.

## Consequences

- CLI users can audit migration provenance by tool version and outcome.
- Migration list queries can find matching entries without lower-level storage
  inspection.
- The change stays inside the Store infrastructure provenance boundary.
