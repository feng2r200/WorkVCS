# ADR-0334: Phase 4HW Record Relation Show Expected State Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs record relation-show` renders a Record-to-Record relation snapshot,
including the relation state digest. Record graph tooling needs a direct
assertion that a relation id resolves to the expected edge version.

## Decision

`workvcs record relation-show` accepts optional `--expected-state-digest HEX`.

The CLI resolves the target commit through the existing query rules, reads the
Record relation snapshot through the Engine facade, parses the expected digest
through the core `Digest` parser, and returns `matches_expected=true` only when
the relation state digest matches. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a Record relation id resolves to an unexpected edge,
label, or version. The command does not change Record relation creation,
removal, restore, list filtering, replay, or relation digest semantics.
