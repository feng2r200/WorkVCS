# ADR-0329: Phase 4HR Verification Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs verification show` renders a Verification semantic snapshot at either
a branch head or an explicit commit, including its state digest. Evidence
closure tooling needs a direct assertion that the verification record being
consumed is the expected semantic version.

## Decision

`workvcs verification show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Verification snapshot through the Engine facade, parses the expected digest
through the core `Digest` parser, and returns `matches_expected=true` only when
the Verification state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Verification id resolves to an unexpected result,
method, evidence set, or resource basis. The command does not change
Verification recording, list filtering, replay, or state digest semantics.
