# ADR-0409: Phase 4KT Bundle Import List Branch Detail

Status: Accepted
Date: 2026-08-31

## Context

ADR-0408 made `bundle import-show` recover persisted branch-head detail from
`import_attempt_outcome.detail_json`. That solves single import-attempt
inspection, but operators still need to scan an import queue before choosing
which attempt to inspect in detail.

`bundle import-list` already lists persisted import attempts and can filter by
Bundle digest, source Store, and outcome. It currently renders each attempt's
outcome and detail digest metadata but not the branch-head detail summary that
is already persisted in the same outcome snapshot.

## Decision

Phase 4KT extends the CLI `bundle import-list` output to render persisted
branch-head detail under each listed import prefix:

- `import.N.branch_head_detail_count`;
- `import.N.branch_head_detail.M.*` fields matching ADR-0407 and ADR-0408.

The list command reads the same `BundleImportAttemptSnapshot` outcome projection
that powers `bundle import-show`. It does not add new list filters or
expectation flags in this slice; scripts can continue to use the existing
`--expected-imports` list count check and inspect rendered keys.

This is a persisted observability slice. It does not change import recording,
Bundle preflight classification, Bundle application, Bundle manifest or payload
format, database schema, automatic merge behavior, Branch creation, divergent
commit import, or external Store import.

## Consequences

Import queues with same-Store divergence entries become scan-friendly: a single
`bundle import-list` can show which Branch, source head, target head, status,
and merge base are associated with each persisted attempt.

The safety boundary remains unchanged. Divergent same-Store Bundles stay
non-applied, and import-list remains a read-only query over already persisted
import attempt evidence.
