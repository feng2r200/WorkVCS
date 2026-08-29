# ADR-0169: Phase 4BN ChangeSet Integrity Doctor

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ChangeSet summary and operation drill-down queries now validate stored
canonical JSON, typed identifiers, digests, and operation ordering at read
time. `doctor` should reuse those read paths so repository health checks cover
the same provenance surface that the CLI can inspect manually.

## Decision

1. Extend `IntegrityReport` with checked ChangeSet and change operation counts.
2. `validate_integrity` now enumerates all ChangeSets and validates each through
   `changeset` and `changeset_operations`.
3. CLI `doctor` renders the new counts.

## Non-Goals

- This slice does not add repair, migration, quarantine, or rollback behavior.
- This slice does not alter ChangeSet, change_operation, commit, Event, or
  persistence schema write paths.

## Consequences

- `doctor` now catches non-canonical ChangeSet and change_operation payloads.
- Existing read-side query validation becomes part of the one-command store
  integrity check.
