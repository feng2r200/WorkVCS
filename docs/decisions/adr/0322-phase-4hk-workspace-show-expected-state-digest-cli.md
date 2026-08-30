# ADR-0322: Phase 4HK Workspace Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs workspace show` renders the authoritative Workspace metadata,
including the initial WorkState digest. Scripts that bootstrap or inspect a
workspace need a direct way to assert that the workspace still has the expected
genesis state.

## Decision

`workvcs workspace show` accepts optional `--expected-state-digest HEX`.

The CLI reads the Workspace metadata through the existing Engine facade, parses
the expected digest through the core `Digest` parser, and returns
`matches_expected=true` only when the workspace state digest matches. A mismatch
returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a workspace id resolves to an unexpected initial
WorkState digest or when a copied digest is wrong. The command does not change
workspace creation, branch creation, genesis commits, or WorkState digest
semantics.
