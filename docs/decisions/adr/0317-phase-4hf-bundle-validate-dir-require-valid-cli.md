# ADR-0317: Phase 4HF Bundle Validate Dir Require Valid CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle validate-dir` validates a local bundle payload directory and
reports `valid=true` or `valid=false`. The reporting mode is useful for manual
inspection, while scripts need a fail-fast mode before transfer or import.

## Decision

`workvcs bundle validate-dir` accepts optional `--require-valid`.

Without this flag, the command preserves the existing reporting behavior. With
this flag, `valid=false` is returned as `QueryInvalid`; `valid=true` appends
`valid_required=true`.

## Consequences

Manual workflows can still inspect invalid bundle directories without an error,
while scripts can opt into fail-fast validation. The command does not change
bundle payload construction, manifest validation, preflight, import, or apply
semantics.
