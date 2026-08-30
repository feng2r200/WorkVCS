# ADR-0333: Phase 4HV Knowledge Relation Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs knowledge relation-show` renders a Knowledge relation snapshot,
including the relation state digest. Knowledge graph tooling needs a direct
assertion that a relation id resolves to the expected relation version.

## Decision

`workvcs knowledge relation-show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Knowledge relation snapshot through the Engine facade, parses the expected
digest through the core `Digest` parser, and returns `matches_expected=true`
only when the relation state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Knowledge relation id resolves to an unexpected
edge or version. The command does not change Knowledge relation creation,
removal, restore, list filtering, replay, or relation digest semantics.
