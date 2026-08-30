# ADR-0328: Phase 4HQ Verification Requirement Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs vr show` renders a Verification Requirement semantic snapshot at
either a branch head or an explicit commit, including its state digest.
Evidence tooling needs a direct assertion that the requirement being checked is
the intended version.

## Decision

`workvcs vr show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Verification Requirement snapshot through the Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`matches_expected=true` only when the Verification Requirement state digest
matches. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Verification Requirement id resolves to an
unexpected statement or parent criterion. The command does not change
Verification Requirement creation, revision, list filtering, replay, or state
digest semantics.
