# ADR-0378: Phase 4JO Bundle Import List Outcome Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle import-list` can already filter import attempts by source Store
and Bundle digest. Import outcomes are also persisted in the import attempt
journal, but callers cannot list only attempts with a specific outcome.

## Decision

`BundleImportAttemptListOptions` adds an optional outcome filter. The query
matches the persisted `import_attempt_outcome.outcome` when the filter is
supplied.

The CLI adds:

- `workvcs bundle import-list --outcome OUTCOME`

The existing `--expected-imports` check applies after the outcome filter.

## Consequences

Bundle diagnostics can inspect only attempts with a requested outcome, such as
`already_present` or `same_store_fast_forward_applied`, without direct SQL. This
does not change import attempt recording, outcome persistence, preflight
classification, bundle application, or storage schema.
