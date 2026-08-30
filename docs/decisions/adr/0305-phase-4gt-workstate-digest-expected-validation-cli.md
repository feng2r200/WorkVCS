# ADR-0305: Phase 4GT WorkState Digest Expected Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs canonical work-state-digest` computes the CE-13 WorkState mapping
digest from typed entity and relation mappings. Manual and scripted validation
often needs to compare that computed digest against an expected digest from a
Commit, checkpoint, manifest, or external note.

## Decision

`workvcs canonical work-state-digest` accepts optional `--expected-digest HEX`.

The CLI computes the WorkState mapping digest through the existing core
canonical function, parses the expected digest through the core `Digest` parser,
and returns `matches_expected=true` only when the values match. A mismatch
returns `DigestInvalid`.

## Consequences

Scripts can fail fast on WorkState digest mismatches without reimplementing
CE-13 comparison logic. The command does not change WorkState mapping digest
semantics, stored Commit digests, or canonical hashing domains.
