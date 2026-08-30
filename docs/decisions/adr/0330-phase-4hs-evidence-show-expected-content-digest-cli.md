# ADR-0330: Phase 4HS Evidence Show Expected Content Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs evidence show` renders an Evidence capture and its attached content
object digests. Tooling that consumes Evidence ids often needs to assert that
the Evidence still contains a specific raw-byte content object.

## Decision

`workvcs evidence show` accepts optional `--expected-content-digest HEX`.

The CLI reads the Evidence snapshot through the Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`content_matches_expected=true` only when one rendered Evidence content object
has the expected digest. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when an Evidence id resolves to the wrong capture or when
the expected raw content object is absent. The command does not change Evidence
capture, content digest calculation, list filtering, or verification semantics.
