# ADR-0315: Phase 4HD Bundle Export Expected Digests CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle export` renders a local export manifest summary, including the
target WorkState digest and manifest digest. Manual and scripted export flows
need to assert those digests before handing the manifest to follow-up validation
or transfer steps.

## Decision

`workvcs bundle export` accepts optional `--expected-state-digest HEX` and
`--expected-manifest-digest HEX`.

The CLI computes the export manifest through the existing Engine facade, parses
expected digests through the core `Digest` parser, and returns
`state_matches_expected=true` or `manifest_matches_expected=true` for each
matching expectation supplied. A mismatch returns `DigestInvalid`.

## Consequences

Scripts can fail fast when a commit resolves to an unexpected WorkState digest
or when the generated manifest digest differs from the expected value. The
command does not change bundle manifest construction, canonical manifest JSON,
payload export, or import validation semantics.
