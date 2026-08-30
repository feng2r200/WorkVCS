# ADR-0307: Phase 4GV Content Digest Expected Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs canonical content-digest` computes raw-byte ContentObject digests from
text, hex, or file input. Manual workflows often need to verify those bytes
against a digest copied from Evidence, checkpoint, bundle, or resource
provenance.

## Decision

`workvcs canonical content-digest` accepts optional `--expected-digest HEX`.

The command computes the raw-byte ContentObject digest using the existing core
function, parses the expected value through the core `Digest` parser, and
returns `matches_expected=true` only when the values match. A mismatch returns
`DigestInvalid`.

## Consequences

Scripts can fail fast on raw content digest mismatches without reimplementing
ContentObject digest logic. The command does not alter raw-byte digest
semantics, content storage, or canonical JSON behavior.
