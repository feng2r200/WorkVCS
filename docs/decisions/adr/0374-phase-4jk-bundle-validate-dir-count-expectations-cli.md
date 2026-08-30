# ADR-0374: Phase 4JK Bundle Validate-Dir Count Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle validate-dir` validates a deterministic bundle payload directory
against the exported manifest and payload index. The command already reports
payload file and payload reference counts, but acceptance scripts had to parse
them externally.

## Decision

The CLI adds optional count expectations to `workvcs bundle validate-dir`:

- `--expected-payload-files COUNT`
- `--expected-payload-references COUNT`

The command continues to use the existing Engine validation path. The payload
file expectation checks `actual_payload_files`. The payload reference
expectation checks the manifest/index expected reference count returned by
validation. Passing checks append `*_match_expected=true` markers. A mismatch
returns `QueryInvalid`.

## Consequences

Bundle directory validation scripts can assert payload directory shape without
external parsing. This does not change payload validation rules, payload index
semantics, bundle export, bundle import, or storage schema.
