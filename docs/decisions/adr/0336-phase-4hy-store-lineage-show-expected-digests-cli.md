# ADR-0336: Phase 4HY Store Lineage Show Expected Digests CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs store lineage-show` renders provenance for a source store derivation,
including the canonical source root descriptor digest and optional source bundle
digest. Automation needs to assert these provenance digests without parsing and
comparing the fields outside the CLI.

## Decision

`workvcs store lineage-show` accepts optional
`--expected-source-root-descriptor-digest HEX` and
`--expected-source-bundle-digest HEX`.

The CLI reads the lineage snapshot through the Engine facade, parses expected
digests through the core `Digest` parser, and returns
`source_root_descriptor_matches_expected=true` or
`source_bundle_matches_expected=true` for each supplied matching expectation. A
missing source bundle or digest mismatch returns `DigestInvalid`.

## Consequences

Store provenance scripts can fail fast when a lineage id points to an unexpected
source root descriptor or source bundle. The command does not change lineage
recording, listing, bundle import/export, or canonical digest semantics.
