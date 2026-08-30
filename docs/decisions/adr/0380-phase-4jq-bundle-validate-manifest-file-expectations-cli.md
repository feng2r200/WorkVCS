# ADR-0380: Phase 4JQ Bundle Validate Manifest File Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs bundle validate-manifest` validates a manifest file against a target
Commit and reports the actual manifest digest and file size. Scripts need to
assert those file-level values directly when checking deterministic bundle
artifacts.

## Decision

The CLI adds optional file expectations to `workvcs bundle validate-manifest`:

- `--expected-actual-manifest-digest DIGEST`
- `--expected-manifest-size-bytes BYTES`

The command continues to validate through the existing Engine path. Passing
checks append `actual_manifest_matches_expected=true` or
`manifest_size_matches_expected=true`. A digest mismatch returns
`DigestInvalid`; a size mismatch returns `QueryInvalid`.

## Consequences

Bundle manifest files can be verified as deterministic artifacts without
parsing command output outside WorkVCS. This does not change manifest
construction, validation semantics, bundle payload export, import preflight, or
storage schema.
