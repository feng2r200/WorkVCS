# ADR-0342: Phase 4IE Record Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs record show` renders a Record snapshot and its state digest. Scripts
that inspect governance records need to assert that a Record id resolves to the
expected Record version.

## Decision

`workvcs record show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Record snapshot through the Engine facade, parses the expected digest through
the core `Digest` parser, and returns `matches_expected=true` only when the
Record state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Record inspection scripts can fail fast when a Record id resolves to an
unexpected version. The command does not change Record creation, status
transition, list filtering, replay, or digest semantics.
