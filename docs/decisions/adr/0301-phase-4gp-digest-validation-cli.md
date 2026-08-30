# ADR-0301: Phase 4GP Digest Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

The canonical CLI can compute semantic and raw content digests. Manual
workflows also need to validate digest text copied from logs, manifests, or
other CLI output without opening a Store.

## Decision

`workvcs canonical digest-validate --digest HEX` validates the supplied digest
through the core `Digest` parser.

The command accepts only the confirmed BLAKE3-256 lowercase hex digest text and
returns the canonical digest plus `valid=true` on success.

## Consequences

CLI users can distinguish valid WorkVCS digest text from malformed input without
writing Rust code or touching SQLite state. The command does not compute a new
digest and does not change canonical hashing rules.
