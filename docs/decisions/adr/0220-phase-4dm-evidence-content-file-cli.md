# ADR-0220: Phase 4DM Evidence Content File CLI

Status: Accepted

Date: 2026-08-30

## Context

`workvcs evidence create` can attach Evidence content metadata from inline text
or from a precomputed digest plus size. Manual runs and scripts often already
have Evidence content in files, and should be able to compute the same digest
and size without embedding the content into a command argument.

## Decision

1. Add `--content-file PATH` to `workvcs evidence create`.
2. Keep `--content`, `--content-file`, and `--content-digest` mutually
   exclusive content sources.
3. For `--content-file`, read the file bytes and reuse
   `EvidenceContentInput::from_raw_bytes`.
4. Keep media type and format metadata behavior unchanged.

## Non-Goals

- This slice does not add Evidence blob storage.
- This slice does not add Evidence content retrieval.
- This slice does not change content digest rules.
- This slice does not add multiple content entries in one command.

## Consequences

- Operators can record Evidence content metadata from local files.
- The CLI remains aligned with existing raw-byte ContentObject digest behavior.
- Evidence content remains metadata-only until a separate confirmed storage
  slice exists.
