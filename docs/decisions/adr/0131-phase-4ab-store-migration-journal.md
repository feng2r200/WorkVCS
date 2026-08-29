# ADR-0131: Phase 4AB Store Migration Journal

- **Status:** Accepted for implementation
- **Date:** 2026-08-30

## Context

INV-078 requires Store infrastructure history to be auditable. The v0.1 schema
already contains `store_migration_attempt` and `store_migration_outcome`, and
the physical schema document states that migration attempts structurally record
Store-format/schema from/to versions plus tool version while complete manifest
delta belongs in immutable outcome detail.

## Decision

1. Add a typed `MigrationId` for `store_migration_attempt.migration_id`.
2. Add Engine-level APIs to record, show, and list Store migration audit rows.
3. The record API writes one immutable migration attempt row and one immutable
   outcome row in a single transaction.
4. Attempt fields validate positive Store-format/schema versions and non-empty
   tool version.
5. Outcome detail is a WorkVCS fixed-point canonical JSON object and is
   revalidated on read.
6. CLI adds `workvcs store migration-record`, `workvcs store migration-show`,
   and `workvcs store migration-list`.
7. This slice does not perform a real Store format/schema migration, mutate the
   Store manifest, change SQLite schema, rewrite data, import Bundles, move refs,
   or perform remote synchronization.

## Consequences

Store migration audit history can now be recorded and inspected through the
Engine facade and thin CLI without exposing SQLite handles. A future real
migration executor can reuse this journal while adding its own compatibility and
manifest update rules.

## Implementation Finding

No new contract ambiguity was found. The existing schema already separates
attempt structure from immutable outcome detail, so this slice keeps migration
execution outside scope.
