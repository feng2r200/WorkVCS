# ADR-0283: Phase 4FX Evidence List Content Metadata Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

Evidence content metadata includes role, digest, size, optional media type, and
canonical format metadata. Phase 4FW added digest-based Evidence discovery, but
users still need to find Evidence by common content metadata such as stdout,
stderr, or media type without bypassing the CLI.

## Decision

`workvcs evidence list` accepts:

- `--content-role <ROLE>`
- `--media-type <MEDIA_TYPE>`

When any content metadata filters are supplied, an Evidence row matches only if
at least one of its content entries satisfies every supplied content predicate.
The CLI applies these filters after the Engine Evidence list query and before
`--limit`.

## Consequences

Users can discover Evidence by practical content metadata while keeping the
existing Evidence list output shape and Store-scoped provenance boundary.

This slice does not add raw blob storage, storage-location rows, format
metadata filtering, commit-scoped Evidence projection, new Engine query APIs,
or schema.
