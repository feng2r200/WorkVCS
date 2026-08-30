# ADR-0286: Phase 4GA Verification List Evidence Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

Verification snapshots record the Evidence objects used to support a
Verification judgment, and `verification list` already renders the Evidence
count for each Verification. Users can filter by target kind, target entity,
and result, but cannot answer the reverse question: which Verifications cite a
known Evidence object.

## Decision

`workvcs verification list` accepts `--evidence <EVIDENCE_ID>`.

The CLI parses the supplied value as a typed `EvidenceId`, loads Verification
snapshots through the existing Branch/Commit query path, filters snapshots that
contain the Evidence id in their recorded Evidence set, and then applies any
`--limit`.

## Consequences

Users can trace Evidence reuse across Verification judgments without bypassing
the Engine facade or changing list output shape.

This slice does not change Verification recording, Evidence semantics,
effective projection, resource basis handling, Engine query APIs, or schema.
