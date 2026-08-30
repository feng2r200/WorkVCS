# ADR-0280: Phase 4FU Store Limit Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

Store-scoped query commands expose lineage, migration, external reference,
Knowledge Space, and Knowledge Space exposure projections. These commands
already accept `--limit <N>` and pass positive values into Engine query options,
but zero-limit rejection was left to lower layers with inconsistent messages
and after unnecessary store-opening work.

## Decision

The CLI rejects `--limit 0` before opening the target store for these commands:

- `workvcs store lineage-list`
- `workvcs store migration-list`
- `workvcs store external-ref-list`
- `workvcs store knowledge-space-list`
- `workvcs store knowledge-space-available-exposures`
- `workvcs store knowledge-space-source-stale-exposures`
- `workvcs store knowledge-space-historical-exposures`
- `workvcs store knowledge-space-refresh-source-statuses`

Positive limits continue through the existing Engine options and preserve the
current filtering, ordering, rendering, and mutation behavior.

## Consequences

Store query tools now fail fast for invalid zero limits and match the rest of
the bounded CLI surface.

This slice does not change Store schema, lineage semantics, migration records,
external object reference semantics, Knowledge Space lifecycle, exposure
projection rules, or Engine query APIs.
