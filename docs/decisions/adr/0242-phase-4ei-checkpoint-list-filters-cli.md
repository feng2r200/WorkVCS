# ADR-0242: Phase 4EI Checkpoint List Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

Checkpoint inspection is part of the local recovery and portability tool
surface. `workvcs checkpoint list` was scoped to a commit, but did not allow
callers to narrow candidates by usability state or content digest.

The engine already returns checkpoint snapshots for a commit. This slice does
not introduce a new checkpoint state, validation rule, storage query, or schema
change.

## Decision

`workvcs checkpoint list` accepts:

- `--usability-state <STATE>`
- `--content-digest <DIGEST_HEX>`

Both filters are CLI-side filters over the `CheckpointSnapshot` values returned
by the existing engine list call. `--content-digest` must parse as canonical
lowercase BLAKE3-256 digest hex before matching.

When both filters are present, both must match.

## Consequences

Operators can select usable or digest-specific checkpoint candidates while the
engine and storage boundary remain unchanged.

`--usability-state` intentionally matches stored projection text exactly and
does not define new checkpoint lifecycle vocabulary.
