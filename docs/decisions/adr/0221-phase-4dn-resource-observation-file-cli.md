# ADR-0221: Phase 4DN Resource Observation File CLI

Status: Accepted

Date: 2026-08-30

## Context

`workvcs resource observe` can compute a Resource Observation fingerprint from
inline content or accept a precomputed digest. For local tool workflows, the
observed bytes commonly live in a file, and the CLI should compute the same
raw-byte digest without requiring inline content.

## Decision

1. Add `--content-file PATH` to `workvcs resource observe`.
2. Keep `--fingerprint`, `--content`, and `--content-file` mutually exclusive.
3. For `--content-file`, read file bytes and use the existing
   ContentObject raw-byte digest function.
4. Keep Resource Observation summary and adapter validation unchanged.

## Non-Goals

- This slice does not add adapter execution.
- This slice does not add Resource Observation detail-content storage.
- This slice does not change fingerprint or digest rules.
- This slice does not add blob retrieval.

## Consequences

- Resource Observation fingerprints can be recorded from local files.
- Manual drift checks can use the same CLI path as inline content checks.
- Observation recording remains a thin wrapper over the existing Engine API.
