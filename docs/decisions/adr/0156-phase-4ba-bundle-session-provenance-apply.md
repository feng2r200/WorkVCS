# ADR-0156: Phase 4BA Bundle Session Provenance Apply

Status: Accepted
Date: 2026-08-30

## Context

Phase 4AZ added Bundle export/apply support for Verification object closure,
including Evidence, Resource, ResourceObservation, and Verification basis rows.
It intentionally rejected Evidence and ResourceObservation rows with
`source_session_id` because Session provenance was not yet part of the
same-Store apply document.

Confirmed architecture separates stable Session occurrence provenance from
mutable SessionRuntime coordination state. Imported Sessions must not be
blindly resumed.

## Decision

Phase 4BA extends the local Bundle manifest and same-Store apply document with:

- `sessions`: stable Session occurrence rows referenced by exported Evidence or
  ResourceObservation `source_session_id`;
- `session_diffs`: final ended SessionDiff rows for those Sessions;
- payload roles `session_metadata` and `session_diff_summary`;
- apply result counters `imported_sessions` and `imported_session_diffs`.

Same-Store apply is supported only when every referenced source Session has both
a `sessions` entry and a final `session_diffs` entry. Apply imports the stable
`session` and `session_diff` rows, but it does not import or restore
`session_runtime`, Session focus rows, context rows, claims, or merge runtime.
If the target Store already has active runtime for the imported Session, apply
fails with an immutable import error instead of converting runtime state.

## Consequences

- Evidence and ResourceObservation rows with an ended `source_session_id` can be
  exported, preflighted, and applied through the existing same-Store
  fast-forward path.
- Active source Sessions remain outside this slice's apply scope because the
  schema has no stable imported-active lifecycle representation separate from
  runtime.
- No schema change is required.

## Implementation Findings

- SessionDiff is not tied to a WorkStateCommit, so Bundle export resolves it
  from the referenced source Session rather than the commit closure.
- Session metadata and SessionDiff summary remain canonical JSON payloads in the
  Bundle payload index; raw content bytes are not introduced by this slice.
