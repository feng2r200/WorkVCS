# ADR-0293: Phase 4GH Changeset Provenance List Limits CLI

Status: Accepted
Date: 2026-08-30

## Context

Most WorkVCS CLI list tools support `--limit` and reject zero limits. The
Changeset provenance lists `changeset operations` and `changeset anchors` were
filterable but had no list-size control.

## Decision

`workvcs changeset operations` and `workvcs changeset anchors` accept
`--limit <N>`.

Both commands reject `--limit 0` and apply the limit after any CLI-side filters.
The commands continue to use the existing Engine facade results and keep their
output shape unchanged.

## Consequences

Users can inspect large Changeset provenance lists incrementally without new
Engine APIs, schema changes, or rendering changes.
