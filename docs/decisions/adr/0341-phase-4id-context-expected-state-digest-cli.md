# ADR-0341: Phase 4ID Context Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs context` renders the active branch head and state digest for a session.
Automation needs to assert that the session context still resolves to the
expected WorkState before selecting runnable work.

## Decision

`workvcs context` accepts optional `--expected-state-digest HEX`.

The CLI reads the context overview through the Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`matches_expected=true` only when the context branch state digest matches. A
mismatch returns `DigestInvalid`.

## Consequences

Runtime scripts can fail fast when a session context has moved to an unexpected
branch state. The command does not change session state, claim state, runnable
projection, context rendering, or replay semantics.
