# ADR-0292: Phase 4GG Changeset Anchor Filters CLI

Status: Accepted
Date: 2026-08-30

## Context

`changeset anchors` exposes causal anchor object ids and object kinds. Users can
inspect the full list, but cannot narrow anchors to a specific causal object or
object family from the CLI.

## Decision

`workvcs changeset anchors` accepts:

- `--object <OBJECT_ID>`
- `--object-kind <OBJECT_KIND>`

The command filters the `ChangeSetCausalAnchorListResult` returned by the Engine
facade and preserves the existing output shape.

## Consequences

Users can inspect causal provenance anchors by object id or kind without adding
schema, changing causal anchor storage, or bypassing the Engine facade.
