# ADR-0316: Phase 4HE Bundle Validate Manifest Require Valid CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle validate-manifest` reports whether a manifest matches the
expected export manifest for a commit. The default output is useful for
inspection, but scripts also need a mode where an invalid manifest causes the
command to fail.

## Decision

`workvcs bundle validate-manifest` accepts optional `--require-valid`.

Without this flag, the command preserves the existing reporting behavior and
prints `valid=true` or `valid=false`. With this flag, `valid=false` is returned
as `QueryInvalid`; `valid=true` appends `valid_required=true`.

## Consequences

Manual workflows can still inspect invalid manifests without an error, while
scripts can opt into fail-fast validation. The command does not change bundle
manifest construction, canonical JSON validation, payload export, or import
semantics.
