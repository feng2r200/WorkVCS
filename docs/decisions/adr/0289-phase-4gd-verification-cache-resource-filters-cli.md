# ADR-0289: Phase 4GD Verification Cache Resource Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

Verification applicability cache entries record resource stamps that explain
whether a cached Verification judgment is still applicable. `cache-list` could
filter by Verification id, applicability, reason code, and limit, but not by the
resource facts carried in the cache.

## Decision

`workvcs verification cache-list` accepts:

- `--resource <RESOURCE_ID>`
- `--observation <RESOURCE_OBSERVATION_ID>`
- `--observed-fingerprint <DIGEST>`

Observation and fingerprint predicates match directly against cache resource
stamps. Resource predicates resolve the cached Verification snapshot at its
evaluated commit and use each stamp's `resource_basis_ordinal` to match the
corresponding Verification resource basis entry. Combined predicates must match
the same resource stamp and, for `--resource`, the same ordinal's basis entry.

## Consequences

Users can find applicability cache records from concrete resource facts without
adding new schema, changing cache recording, or exposing lower-level store
handles through the CLI.
