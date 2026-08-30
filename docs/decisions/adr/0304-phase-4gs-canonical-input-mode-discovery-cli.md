# ADR-0304: Phase 4GS Canonical Input Mode Discovery CLI

Status: Accepted
Date: 2026-08-30

## Context

The canonical CLI accepts different input modes for semantic JSON and raw
content bytes. Manual users and scripts benefit from discovering those supported
flags directly from the tool instead of consulting source code.

## Decision

`workvcs canonical input-modes` lists the input flags accepted by the canonical
JSON commands and the raw content digest command.

The command is store-independent and deterministic. It only reports existing
input modes:

- canonical JSON: `--json`, `--json-file`
- raw content: `--content`, `--content-hex`, `--content-file`

## Consequences

Manual tooling can discover canonical input modes without opening a Store. This
does not add a new input mode, change content digesting, or alter canonical JSON
validation.
