# ADR-0323: Phase 4HL Knowledge Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs knowledge show` renders the authoritative Knowledge semantic snapshot,
including its state digest. Tooling that consumes exported ids or historical
queries needs a direct assertion that the resolved Knowledge state is exactly
the expected one.

## Decision

`workvcs knowledge show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Knowledge snapshot through the Engine facade, parses the expected digest through
the core `Digest` parser, and returns `matches_expected=true` only when the
Knowledge state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Knowledge id resolves to an unexpected version or
when a copied digest is wrong. The command does not change Knowledge creation,
transition, list filtering, replay, or state digest semantics.
