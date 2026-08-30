# ADR-0318: Phase 4HG Bundle Preflight Requirements CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle preflight-dir` reports whether a local bundle directory is valid
and whether it can be applied to the target store. Manual inspection needs the
full report, while scripts need fail-fast checks before invoking apply.

## Decision

`workvcs bundle preflight-dir` accepts optional `--require-valid` and
`--require-can-apply`.

Without these flags, the command preserves the existing reporting behavior.
With `--require-valid`, `valid=false` is returned as `QueryInvalid`; `valid=true`
appends `valid_required=true`. With `--require-can-apply`, `can_apply=false` is
returned as `QueryInvalid`; `can_apply=true` appends
`can_apply_required=true`.

## Consequences

Scripts can require that a bundle directory is both valid and applicable before
running apply. Manual workflows can still inspect already-present, divergent, or
invalid preflight reports without an error. The command does not change bundle
validation, import attempt recording, or apply semantics.
