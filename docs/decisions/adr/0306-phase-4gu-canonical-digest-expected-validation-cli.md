# ADR-0306: Phase 4GU Canonical Digest Expected Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs canonical digest` computes EntityVersion and RelationVersion semantic
digests. Manual workflows often need to compare the computed digest with an
expected value copied from stored history, a manifest, or a conformance vector.

## Decision

`workvcs canonical digest` accepts optional `--expected-digest HEX`.

The command computes the semantic digest using the existing domain-separated
core functions, parses the expected value through the core `Digest` parser, and
returns `matches_expected=true` only when the values match. A mismatch returns
`DigestInvalid`.

## Consequences

Scripts can fail fast on semantic digest mismatches without reimplementing
domain-separated hashing. The command does not add hashing domains or change
canonical JSON, EntityVersion, or RelationVersion digest semantics.
