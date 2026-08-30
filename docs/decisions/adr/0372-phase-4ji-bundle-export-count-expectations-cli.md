# ADR-0372: Phase 4JI Bundle Export Count Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle export` renders the deterministic manifest summary and key
closure counts. Acceptance scripts commonly need to assert the expected bundle
shape before validating or applying exported payloads.

## Decision

The CLI adds optional count expectations to `workvcs bundle export`:

- `--expected-commits COUNT`
- `--expected-exported-branch-heads COUNT`
- `--expected-entities COUNT`
- `--expected-relations COUNT`
- `--expected-checkpoint-candidates COUNT`

The command continues to build the manifest through the existing Engine export
path. Passing checks append `*_match_expected=true` markers. A mismatch returns
`QueryInvalid`.

## Consequences

Bundle export acceptance scripts can assert the core closure shape without
external parsing. This does not change manifest construction, closure rules,
payload export, bundle validation, bundle import, or storage schema.
