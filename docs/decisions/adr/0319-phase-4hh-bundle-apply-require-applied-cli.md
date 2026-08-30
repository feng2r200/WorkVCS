# ADR-0319: Phase 4HH Bundle Apply Require Applied CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle apply-dir` reports whether a local bundle directory was actually
applied to the target store. Scripts that call apply need to distinguish a real
store update from an already-present or otherwise non-applied outcome.

## Decision

`workvcs bundle apply-dir` accepts optional `--require-applied`.

Without this flag, the command preserves the existing reporting behavior. With
this flag, `applied=false` is returned as `QueryInvalid`; `applied=true` appends
`applied_required=true`.

## Consequences

Scripts can fail fast when apply does not advance or update the target store.
Manual workflows can still inspect non-applied outcomes without an error. The
command does not change bundle preflight, import attempt recording, apply
semantics, or branch head movement.
