# ADR-0314: Phase 4HC Projection Show Expected Digests CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs projection show` renders both the Branch HEAD state digest and the
stored projection state digest. Manual and scripted workflows need to check
that a materialized projection still matches the expected branch state before
using the projection as a cached read basis.

## Decision

`workvcs projection show` accepts optional `--expected-head-state-digest HEX`
and `--expected-projection-state-digest HEX`.

The CLI reads the projection snapshot through the existing Engine facade,
parses expected digests through the core `Digest` parser, and returns
`head_state_matches_expected=true` or
`projection_state_matches_expected=true` for each matching expectation supplied.
A mismatch returns `DigestInvalid`. Expecting a projection state digest when no
projection digest is materialized also returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a branch moves, a projection is stale, a projection
is not materialized, or a copied digest is wrong. The command does not change
projection refresh, branch movement, replay, or WorkState digest semantics.
