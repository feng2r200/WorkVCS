# ADR-0313: Phase 4HB Changeset Show Expected Digests CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs changeset show` renders the authoritative ChangeSet snapshot,
including the operation payload digest and rationale digest. Scripts that use a
ChangeSet as a review or replay anchor need a direct way to assert those
digests before continuing.

## Decision

`workvcs changeset show` accepts optional
`--expected-operation-payload-digest HEX` and `--expected-rationale-digest HEX`.

The CLI reads the ChangeSet snapshot through the existing Engine facade, parses
expected digests through the core `Digest` parser, and returns
`operation_payload_matches_expected=true` or
`rationale_matches_expected=true` for each matching expectation supplied. A
mismatch returns `DigestInvalid`.

## Consequences

Manual and scripted workflows can fail fast when a ChangeSet id resolves to an
unexpected operation payload or rationale digest. The command does not change
ChangeSet storage, canonical payload encoding, rationale encoding, history
replay, or changeset operation listing.
