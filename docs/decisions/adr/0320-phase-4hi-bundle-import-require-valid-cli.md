# ADR-0320: Phase 4HI Bundle Import Require Valid CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle import-dir` records a bundle import attempt and reports the
preflight validity of the local bundle directory. Manual inspection can use the
reported attempt even when the bundle is invalid, while scripts need a way to
reject invalid bundles immediately.

## Decision

`workvcs bundle import-dir` accepts optional `--require-valid`.

Without this flag, the command preserves the existing reporting behavior. With
this flag, `valid=false` is returned as `QueryInvalid`; `valid=true` appends
`valid_required=true`.

## Consequences

Scripts can fail fast on invalid bundle directories while still using
`import-dir` as the import-attempt recording command. Manual workflows can
continue to inspect invalid attempts without this flag. The command does not
change preflight semantics, import attempt persistence, bundle application, or
branch movement.
