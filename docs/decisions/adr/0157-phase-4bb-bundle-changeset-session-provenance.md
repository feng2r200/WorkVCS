# ADR-0157: Phase 4BB Bundle Changeset Session Provenance

Status: Accepted
Date: 2026-08-30

## Context

Phase 4BA added same-Store Bundle export/apply support for stable Session
provenance referenced by Evidence and ResourceObservation rows.

The physical schema also records `changeset.origin_session_id`. Without explicit
Bundle handling, commit closure export/import would reconstruct imported
Changeset rows with a `NULL` origin even when the source Store had preserved a
Session origin.

## Decision

Phase 4BB extends Bundle commit closure rows with nullable `origin_session_id`.
Export includes any referenced origin Session in the Session provenance closure
and includes the ended SessionDiff required by the same-Store immutable import
contract.

Same-Store apply requires every `changeset.origin_session_id` referenced by an
imported commit to be present in `sessions` and covered by an ended
`session_diffs` row. Apply inserts the imported Changeset row with the original
origin Session id after the Session row has been imported or verified.

## Consequences

- Bundle apply no longer drops stable Changeset Session provenance.
- The public high-level Task and WorkState commit APIs remain unchanged in this
  slice.
- Active origin Sessions remain unsupported by same-Store apply; imported
  provenance must have an ended SessionDiff.
- No schema change is required.

## Implementation Findings

- Existing high-level commit creation paths still write
  `changeset.origin_session_id = NULL` even though the schema supports it. This
  slice preserves existing non-null data first so future producer changes do not
  lose provenance during Bundle transfer.
- The focused conformance test seeds `origin_session_id` directly through the
  Store because no confirmed public operation option currently exposes it.
