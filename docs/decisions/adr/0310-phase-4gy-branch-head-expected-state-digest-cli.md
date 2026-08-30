# ADR-0310: Phase 4GY Branch Head Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs branch head` renders the current Branch HEAD, including its state
digest. Manual workflows often need to confirm that the branch still points to
the expected WorkState before running follow-up commands.

## Decision

`workvcs branch head` accepts optional `--expected-state-digest HEX`.

The CLI reads the Branch HEAD through the existing Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`matches_expected=true` only when the head state digest matches. A mismatch
returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Branch HEAD has moved or when a copied digest is
wrong. The command does not change Branch movement, replay, projection refresh,
or canonical state digest semantics.
