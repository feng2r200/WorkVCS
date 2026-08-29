# ADR-0151: Phase 4AV Bundle Same-Store Apply Gate

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0149 made Bundle preflight classify exported Branch heads, and ADR-0150
made the manifest graph self-consistent before local comparison. The next
import step needs a deterministic gate that distinguishes a same-Store
fast-forward artifact from still-unsupported import cases.

## Decision

1. Phase 4AV lets Bundle import preflight report a same-Store fast-forward
   Bundle as `can_apply=true` with action `same_store_fast_forward_ready`.
2. The gate is true only when:
   - the Bundle format is compatible with the local Store;
   - the source Store id matches the local Store id;
   - the incoming target Commit is not already present locally;
   - the manifest exports at least one Branch head;
   - at least one exported Branch head is a fast-forward of the local Branch;
   - no exported Branch head is missing locally; and
   - no exported Branch head is diverged.
3. Existing same-Store artifacts that are already present continue to report
   `already_present`.
4. External Store import, missing local Branch refs, diverged refs, and other
   same-Store shapes remain non-applicable in this slice.
5. This slice does not ingest canonical rows, create or move Branch refs,
   activate imports, resolve divergence, or implement cross-Store import.

## Consequences

- Tooling can now tell the difference between "validated but unsupported" and
  "validated and ready for the same-Store fast-forward apply path."
- The next slice can implement actual same-Store import activation behind this
  preflight gate without changing Bundle artifact validation.

## Implementation Findings

- None.
