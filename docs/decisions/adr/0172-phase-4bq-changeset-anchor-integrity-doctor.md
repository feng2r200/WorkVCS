# ADR-0172: Phase 4BQ ChangeSet Anchor Integrity Doctor

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

ADR-0170 added read access for generic ChangeSet causal anchors. ADR-0171 writes
one generic anchor for decision supersede operations that use `--because-record`.
The store-wide doctor should validate that same read surface so corrupt anchor
rows are caught by the normal integrity command.

## Decision

1. Extend `IntegrityReport` with `checked_changeset_causal_anchors`.
2. During `validate_integrity`, each ChangeSet now validates its generic causal
   anchors through `changeset_causal_anchors`.
3. CLI `doctor` renders the new count.

## Non-Goals

- This slice does not add repair, migration, quarantine, or backfill behavior.
- This slice does not add a generic causal anchor write API.
- This slice does not change schema, replay, bundle export, or bundle import.

## Consequences

- `doctor` now catches invalid generic ChangeSet causal anchor identifiers and
  reports how many anchors were checked.
- The generic anchor write/read path added in ADR-0170 and ADR-0171 is covered by
  the same one-command integrity check as ChangeSets and ChangeOperations.
