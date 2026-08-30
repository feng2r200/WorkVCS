# ADR-0338: Phase 4IA External Ref Show Expected Descriptor Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs store external-ref-show` renders an external object reference and the
canonical descriptor digest. Automation needs to assert that an external ref id
still points to the expected descriptor.

## Decision

`workvcs store external-ref-show` accepts optional
`--expected-descriptor-digest HEX`.

The CLI reads the external object reference snapshot through the Engine facade,
parses the expected digest through the core `Digest` parser, and returns
`descriptor_matches_expected=true` only when the descriptor digest matches. A
mismatch returns `DigestInvalid`.

## Consequences

External provenance scripts can fail fast when an external ref id resolves to an
unexpected descriptor. The command does not change external ref recording,
idempotency, listing, or canonical digest semantics.
