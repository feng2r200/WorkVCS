# ADR-0294: Phase 4GI History Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs history` renders commit ids, Changeset ids, commit kinds, operation
types, and WorkState digests, but the command only accepted a start point and
an optional limit. Users had to scan the history output manually to locate a
specific Changeset, operation family, or state digest.

## Decision

`workvcs history` accepts:

- `--changeset <CHANGESET_ID>`
- `--commit-kind <KIND>`
- `--operation <OPERATION_TYPE>`
- `--state-digest <DIGEST>`

When any filter is supplied, the CLI loads the full history for the selected
start point, applies filters, and then applies `--limit`. Without filters, the
existing Engine-side limit path is retained.

## Consequences

Users can narrow history from the CLI without schema changes or new Engine
query APIs. Filtered history avoids early limiting so matching entries are not
lost before filtering.
