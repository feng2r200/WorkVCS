# ADR-0282: Phase 4FW Evidence List Content Digest Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

Evidence can reference zero or more digest-identified ContentObjects through
`evidence_content`. The CLI can create Evidence with content metadata and list
Evidence by kind or source Session, but users cannot discover Evidence that
reuses a known ContentObject digest without bypassing the Engine facade.

## Decision

`workvcs evidence list` accepts `--content-digest <DIGEST>`.

The CLI parses the supplied digest with the existing lowercase hex digest
rules, lists Evidence through the current Engine API, filters the returned
Evidence snapshots to those whose content metadata contains the digest, and
then applies any `--limit`.

## Consequences

Users can find reusable Evidence by ContentObject digest while preserving the
existing Evidence list output shape and Store-scoped provenance boundary.

This slice does not add raw blob storage, storage-location rows, commit-scoped
Evidence projection, new Engine query APIs, or schema.
