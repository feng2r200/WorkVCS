# ADR-0332: Phase 4HU Checkpoint Show Expected Digests CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs checkpoint show` renders checkpoint metadata including the target
WorkState digest and checkpoint payload content digest. Existing validation can
recalculate checkpoint integrity, but scripts also need a direct query-side
assertion that the shown checkpoint is the expected artifact.

## Decision

`workvcs checkpoint show` accepts optional `--expected-state-digest HEX` and
`--expected-content-digest HEX`.

The CLI reads the checkpoint through the Engine facade, parses expected digests
through the core `Digest` parser, and returns `state_matches_expected=true` or
`content_matches_expected=true` only for matching digests. A mismatch returns
`DigestInvalid`.

## Consequences

Scripts can fail fast when a checkpoint id resolves to the wrong target state
or payload object. The command does not change checkpoint creation, validation,
list filtering, latest selection, or checkpoint payload semantics.
