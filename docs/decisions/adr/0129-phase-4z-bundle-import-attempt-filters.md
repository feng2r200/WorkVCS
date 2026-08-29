# ADR-0129: Phase 4Z Bundle Import Attempt Filters

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4Y import attempt query surface.

## Context

Phase 4Y makes Bundle import attempts inspectable through Engine and CLI list
commands. A Store can accumulate many import preflight outcomes over time, and
tooling needs to focus that journal by source Store or deterministic Bundle
artifact identity.

## Decision

1. Phase 4Z extends `BundleImportAttemptListOptions` with optional
   `source_store_id` and `bundle_digest` filters.
2. The list query applies both filters conjunctively when both are supplied.
3. CLI `workvcs bundle import-list` adds `--source-store` and
   `--bundle-digest`.
4. Filtering only narrows the existing immutable import attempt journal; it
   does not change import recording, ingestion, lineage, or activation
   semantics.

## Consequences

- Agents can inspect import history for one source Store or one Bundle artifact
  without direct SQL.
- Future import and lineage slices can reuse the same query surface for local
  diagnostics.

## Implementation Findings

- No new contract ambiguity was found. The filters operate on already-recorded
  immutable infrastructure provenance.
