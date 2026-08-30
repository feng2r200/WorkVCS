# ADR-0312: Phase 4HA Event Show Expected Payload Digest CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs event show` renders the authoritative Event snapshot, including its
payload digest. Manual and scripted workflows need a direct way to assert that
an event id resolves to the expected payload before using it as evidence for
history or replay checks.

## Decision

`workvcs event show` accepts optional `--expected-payload-digest HEX`.

The CLI reads the Event snapshot through the existing Engine facade, parses the
expected digest through the core `Digest` parser, and returns
`matches_expected=true` only when the event payload digest matches. A mismatch
returns `DigestInvalid`.

## Consequences

Scripts can fail fast when an event id points to an unexpected payload digest or
when a copied digest is wrong. The command does not change Event storage,
canonical payload encoding, history replay, or event list filtering.
