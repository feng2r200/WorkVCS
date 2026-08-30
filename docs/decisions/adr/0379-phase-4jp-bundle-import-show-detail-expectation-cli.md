# ADR-0379: Phase 4JP Bundle Import Show Detail Expectation CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle import-show` reports the deterministic digest of an import
attempt outcome detail. Scripts can already check Bundle digest and outcome,
but cannot require a specific detail digest.

## Decision

The CLI adds:

- `workvcs bundle import-show --expected-detail-digest DIGEST`

The command reads the existing import attempt snapshot through the Engine. When
the expectation is supplied, the stored outcome detail digest must match.
Passing checks append `detail_matches_expected=true`. A mismatch returns
`DigestInvalid`; a missing outcome is reported as `none`.

## Consequences

Import attempt detail verification becomes scriptable without parsing the
canonical detail JSON. This does not change import attempt recording, outcome
persistence, preflight classification, bundle application, or storage schema.
