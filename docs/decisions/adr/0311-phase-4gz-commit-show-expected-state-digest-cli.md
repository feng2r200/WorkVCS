# ADR-0311: Phase 4GZ Commit Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs commit show` renders the authoritative Commit snapshot, including the
Commit WorkState digest. Manual and scripted workflows often need to assert
that a copied commit id still resolves to the expected state before using that
snapshot as a basis for follow-up commands.

## Decision

`workvcs commit show` accepts optional `--expected-state-digest HEX`.

The CLI reads the Commit snapshot through the existing Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`matches_expected=true` only when the commit state digest matches. A mismatch
returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a commit id points to an unexpected WorkState digest
or when a copied digest is wrong. The command does not change Commit storage,
history replay, WorkState digest semantics, or branch movement.
