# ADR-0381: Phase 4JR Bundle Import Show Detail Size Expectation CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle import-show` reports the persisted outcome detail size in
bytes. ADR-0379 made the detail digest script-verifiable; callers also need a
direct guard for the stored detail size.

## Decision

The CLI adds:

- `workvcs bundle import-show --expected-detail-size-bytes BYTES`

The command reads the existing import attempt snapshot through the Engine. When
the expectation is supplied, the stored outcome detail size must match. Passing
checks append `detail_size_matches_expected=true`. A mismatch returns
`QueryInvalid`; a missing outcome is reported as `none`.

## Consequences

Import attempt detail size verification becomes scriptable alongside detail
digest verification. This does not change import attempt recording, outcome
persistence, preflight classification, bundle application, or storage schema.
