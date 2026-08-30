# ADR-0337: Phase 4HZ Store Migration Show Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs store migration-show` renders migration audit details, including the
canonical detail digest and migration outcome. Automation needs a direct
assertion that a migration id resolves to the expected audit result.

## Decision

`workvcs store migration-show` accepts optional `--expected-detail-digest HEX`
and `--expected-outcome VALUE`.

The CLI reads the migration snapshot through the Engine facade, parses the
expected detail digest through the core `Digest` parser, and returns
`detail_matches_expected=true` only when the completed migration detail digest
matches. It returns `outcome_matches_expected=true` only when the rendered
outcome matches the expected value. A missing detail object or digest mismatch
returns `DigestInvalid`; an outcome mismatch returns `QueryInvalid`.

## Consequences

Migration audit scripts can fail fast when a migration id points to an
unexpected detail payload or outcome. The command does not change migration
recording, listing, schema validation, or canonical digest semantics.
