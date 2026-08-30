# ADR-0373: Phase 4JJ Bundle Export-Dir Count Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle export-dir` writes the deterministic manifest, payload index,
and payload files to a local directory. It also reports the payload file and
payload reference counts that describe the exported directory shape.

## Decision

The CLI adds optional count expectations to `workvcs bundle export-dir`:

- `--expected-payload-files COUNT`
- `--expected-payload-references COUNT`

The command builds the export through the existing Engine path, checks the
expectations before writing the output directory, and appends
`payload_files_match_expected=true` or
`payload_references_match_expected=true` when the count matches. A mismatch
returns `QueryInvalid` and does not write the target directory.

## Consequences

Bundle payload export scripts can validate the directory shape before writing
local artifacts. This does not change payload selection, payload index
construction, directory layout, bundle validation, bundle import, or storage
schema.
