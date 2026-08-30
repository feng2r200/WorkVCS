# ADR-0309: Phase 4GX Show-At Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs show-at` replays a Branch head or Commit and renders the resulting
WorkState digest. Manual validation often needs to confirm that replayed state
matches an expected digest from another command, checkpoint, manifest, or note.

## Decision

`workvcs show-at` accepts optional `--expected-state-digest HEX`.

The CLI resolves and replays through the existing Engine facade, parses the
expected value through the core `Digest` parser, and returns
`matches_expected=true` only when the replayed state digest matches. A mismatch
returns `DigestInvalid`.

## Consequences

Scripts can fail fast on replay digest mismatches without reimplementing
WorkState digest comparison. The command does not alter replay, Commit storage,
Branch HEAD behavior, or canonical state digest semantics.
