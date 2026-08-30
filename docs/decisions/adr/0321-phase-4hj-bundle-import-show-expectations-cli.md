# ADR-0321: Phase 4HJ Bundle Import Show Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle import-show` renders the persisted import attempt snapshot,
including the bundle digest and recorded outcome. Scripts that inspect an import
attempt need to assert that the record is for the expected bundle and outcome.

## Decision

`workvcs bundle import-show` accepts optional `--expected-bundle-digest HEX` and
`--expected-outcome TEXT`.

The CLI reads the import attempt through the existing Engine facade, parses the
expected bundle digest through the core `Digest` parser, and returns
`bundle_matches_expected=true` or `outcome_matches_expected=true` for each
matching expectation supplied. A digest mismatch returns `DigestInvalid`; an
outcome mismatch returns `QueryInvalid`.

## Consequences

Scripts can fail fast when an import id resolves to the wrong bundle digest or
outcome. The command does not change import attempt persistence, preflight,
apply, or bundle digest semantics.
