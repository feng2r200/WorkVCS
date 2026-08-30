# ADR-0324: Phase 4HM Goal Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs goal show` renders a Goal semantic snapshot at either a branch head or
an explicit commit, including its state digest. External scripts need a direct
way to assert that a resolved Goal snapshot is the intended version.

## Decision

`workvcs goal show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Goal snapshot through the Engine facade, parses the expected digest through the
core `Digest` parser, and returns `matches_expected=true` only when the Goal
state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Goal id resolves to an unexpected lifecycle state
or when a copied digest is wrong. The command does not change Goal creation,
transition, list filtering, replay, or state digest semantics.
