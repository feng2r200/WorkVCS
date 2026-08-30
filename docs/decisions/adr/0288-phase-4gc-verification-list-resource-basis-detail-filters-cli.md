# ADR-0288: Phase 4GC Verification List Resource Basis Detail Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`verification list --resource` can find Verifications that cite a Resource in
their resource basis, but resource-backed workflows also need to trace the
exact baseline observation or fingerprint that made a Verification applicable
or stale.

## Decision

`workvcs verification list` accepts:

- `--baseline-observation <RESOURCE_OBSERVATION_ID>`
- `--baseline-fingerprint <DIGEST>`

When `--resource`, `--baseline-observation`, or `--baseline-fingerprint` are
combined, the CLI requires a single Verification resource basis entry to satisfy
all supplied resource-basis predicates. The command still loads Verification
snapshots through the existing Branch/Commit query path and applies any
`--limit` after filtering.

## Consequences

Users can trace Verification judgments from concrete Resource Observation and
fingerprint facts without changing the Engine facade, schema, applicability
cache semantics, or Verification recording behavior.
