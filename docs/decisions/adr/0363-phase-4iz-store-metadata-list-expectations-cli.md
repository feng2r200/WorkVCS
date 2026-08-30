# ADR-0363: Phase 4IZ Store Metadata List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

Store metadata workflows expose read-only list surfaces for store lineage,
store migrations, and external object references. These commands already apply
scoped filters and optional limits, but automation needs direct count
assertions on the rendered result sets.

## Decision

The CLI adds the following optional count expectations:

- `workvcs store lineage-list --expected-lineages COUNT`
- `workvcs store migration-list --expected-migrations COUNT`
- `workvcs store external-ref-list --expected-external-refs COUNT`

Each command applies existing filters and limits, renders the same list output,
and appends the corresponding `*_match_expected=true` marker when the rendered
count equals the supplied expectation. A mismatch returns `QueryInvalid`.

## Consequences

Store metadata validation scripts can fail fast when scoped lineage, migration,
or external reference results differ from the expected shape. These commands do
not change lineage recording, migration recording, external reference
idempotency, descriptor digests, or list filtering behavior.
