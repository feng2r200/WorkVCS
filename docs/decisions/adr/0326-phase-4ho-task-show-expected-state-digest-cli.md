# ADR-0326: Phase 4HO Task Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs task show` renders a Task semantic snapshot at either a branch head or
an explicit commit, including its state digest. Runnable-work and scheduling
tooling need a direct assertion that a resolved Task snapshot is the expected
version.

## Decision

`workvcs task show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Task snapshot through the Engine facade, parses the expected digest through the
core `Digest` parser, and returns `matches_expected=true` only when the Task
state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Task id resolves to an unexpected lifecycle state
or when a copied digest is wrong. The command does not change Task creation,
transition, list filtering, runnable projections, replay, or state digest
semantics.
