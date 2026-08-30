# ADR-0308: Phase 4GW Canonical Encode Expected JSON CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs canonical encode` renders WorkVCS canonical semantic JSON bytes. Manual
conformance-vector checks often need to compare those bytes with an expected
canonical JSON string and fail directly when they differ.

## Decision

`workvcs canonical encode` accepts optional `--expected-canonical-json JSON`.

The command canonicalizes the input through the existing core parser/encoder
and compares the resulting text exactly with the supplied expected text. A
semantically equivalent but non-canonical expected string does not match.

## Consequences

Scripts can run canonical JSON conformance checks without reimplementing byte
comparison logic. The command does not alter canonical encoding rules or add a
new JSON input mode.
