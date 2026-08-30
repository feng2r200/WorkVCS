# ADR-0201: Phase 4CT Evidence Content CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Evidence may reference ContentObject metadata through `evidence_content`.
Phase 4CS exposed metadata-only Evidence through the CLI but intentionally left
content metadata input deferred. The core Engine already supports Evidence
content inputs backed by digest, size, optional media type, and canonical format
metadata.

## Decision

1. Extend `evidence create` with optional single-content metadata input.
2. Accept either raw CLI `--content` for digest/size calculation or
   `--content-digest` with `--content-size-bytes`.
3. Require `--content-role` whenever any content field is supplied.
4. Accept optional `--media-type` and canonical object
   `--format-metadata-json`.
5. Continue to write only ContentObject digest metadata and Evidence links; raw
   bytes are not stored.

## Non-Goals

- This slice does not support multiple content entries in one command.
- This slice does not add blob storage or storage-location rows.
- This slice does not alter Evidence immutability or Verification closure
  semantics.
- This slice does not add Evidence listing.

## Consequences

- CLI users can create Evidence that references ContentObject metadata.
- Evidence show output can validate the create/show loop for content digest,
  size, media type, and format metadata.

## Implementation Findings

- None.
