# ADR-0325: Phase 4HN Plan Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs plan show` renders a Plan semantic snapshot at either a branch head or
an explicit commit, including its state digest. Scripts that bind follow-up
work to a concrete Plan version need a direct assertion against that digest.

## Decision

`workvcs plan show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Plan snapshot through the Engine facade, parses the expected digest through the
core `Digest` parser, and returns `matches_expected=true` only when the Plan
state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Plan id resolves to an unexpected lifecycle state
or when a copied digest is wrong. The command does not change Plan creation,
transition, list filtering, replay, or state digest semantics.
