# ADR-0335: Phase 4HX Record Knowledge Relation Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs record knowledge-relation-show` renders a Record-to-Knowledge relation
snapshot, including the relation state digest. Record graph tooling needs the
same direct assertion already available for adjacent relation show commands.

## Decision

`workvcs record knowledge-relation-show` accepts optional
`--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Record-to-Knowledge relation snapshot through the Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`matches_expected=true` only when the relation state digest matches. A mismatch
returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Record-to-Knowledge relation id resolves to an
unexpected edge version. The command does not change relation creation, removal,
restore, list filtering, replay, or relation digest semantics.
