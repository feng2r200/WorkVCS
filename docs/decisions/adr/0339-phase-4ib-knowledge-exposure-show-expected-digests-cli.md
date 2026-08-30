# ADR-0339: Phase 4IB Knowledge Exposure Show Expected Digests CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs store knowledge-exposure-show` renders a Knowledge exposure snapshot
with transition detail digest, source Knowledge state digest, and source status
detail digest. Automation needs to assert these provenance and freshness
digests without external field comparison.

## Decision

`workvcs store knowledge-exposure-show` accepts optional
`--expected-transition-detail-digest HEX`,
`--expected-source-knowledge-state-digest HEX`, and
`--expected-source-status-detail-digest HEX`.

The CLI reads the Knowledge exposure snapshot through the Engine facade, parses
expected digests through the core `Digest` parser, and returns one
`*_matches_expected=true` marker for each supplied matching expectation. A
mismatch returns `DigestInvalid`.

## Consequences

Knowledge exposure scripts can fail fast when an exposure id resolves to an
unexpected transition detail, source Knowledge version, or source status detail.
The command does not change exposure creation, withdrawal, refresh, adoption,
listing, or canonical digest semantics.
