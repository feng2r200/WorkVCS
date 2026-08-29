# ADR-0149: Phase 4AT Bundle Branch Preflight

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0148 added `exported_branch_heads` to Bundle manifests so later same-Store
import can compare refs instead of treating a Bundle as only a target Commit.
INV-061 requires same-Store import to distinguish fast-forward from divergence,
and INV-079 forbids partial activation before immutable candidates have been
validated.

## Decision

1. Phase 4AT makes Bundle import preflight parse the exported commit graph and
   exported Branch head refs from the manifest.
2. Preflight compares each exported Branch id with the local Branch table and
   reports counts for already-present, missing, fast-forward, and diverged
   Branch heads.
3. A Branch is classified as fast-forward only when the local head Commit is in
   the exported commit closure, the local head digest matches the manifest, and
   the exported head descends from the local head through manifest parent
   edges.
4. The existing action vocabulary remains unchanged in this slice:
   same-Store artifacts with missing Commits still report
   `same_store_import_not_implemented`.
5. CLI preflight/import-attempt output and import-attempt detail JSON include
   the Branch head comparison counts.
6. This slice does not ingest canonical rows, create or move Branch refs,
   activate imports, resolve divergence, or implement cross-Store import.

## Consequences

- A copied older Store can now inspect a newer same-Store Bundle and see that a
  Branch ref would fast-forward once canonical rows are available locally.
- Later import activation can use these counters and the manifest graph without
  changing the exported Bundle format.

## Implementation Findings

- None.
