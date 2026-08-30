# ADR-0291: Phase 4GF Changeset Operation Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`changeset operations` renders operation ids, subject families, subject object
ids, and operation payload digests, but callers had to scan every operation in a
Changeset manually.

## Decision

`workvcs changeset operations` accepts:

- `--operation <OPERATION_ID>`
- `--subject-family <FAMILY>`
- `--subject-object <OBJECT_ID>`
- `--payload-digest <DIGEST>`

All filters apply to the existing `ChangeOperationSnapshot` list returned by
the Engine facade. The command output shape stays unchanged.

## Consequences

Users can inspect large Changesets by operation identity, operation subject, or
canonical operation payload digest without adding storage/schema/API changes.
