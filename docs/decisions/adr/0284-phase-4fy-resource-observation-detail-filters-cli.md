# ADR-0284: Phase 4FY Resource Observation Detail Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

ResourceObservation can record optional detail content metadata as a
ContentObject digest, byte size, media type, and canonical format metadata.
The CLI can create, show, and list observations, but list filtering only covers
resource, adapter kind, adapter schema version, and source Session.

Verification and resource provenance audits often start from a known detail
content digest or media type, so the CLI needs a direct discovery path without
lower-level storage inspection.

## Decision

`workvcs resource observation-list` accepts:

- `--detail-content-digest <DIGEST>`
- `--detail-media-type <MEDIA_TYPE>`

The CLI parses the digest with existing lowercase hex digest rules, lists
observations through the current Engine API, filters the returned snapshots by
detail content metadata, and then applies any `--limit`.

## Consequences

Users can discover ResourceObservations by detail ContentObject metadata while
preserving the existing Store-level provenance boundary and list output shape.

This slice does not add adapter execution, raw blob storage, storage-location
rows, commit-scoped ResourceObservation projections, new Engine query APIs, or
schema.
