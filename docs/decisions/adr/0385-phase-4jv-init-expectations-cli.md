# ADR-0385: Phase 4JV Init Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs init` creates a Store and reports the generated Store id and schema
version. Phase 4JU made existing Store manifest baselines script-verifiable
through `workvcs store info`. Creation scripts need the same immediate
acceptance gate on the initialization command.

## Decision

The CLI adds optional post-initialization expectations to `workvcs init`:

- `--expected-display-name NAME`
- `--expected-store-format-version VALUE`
- `--expected-schema-version VALUE`
- `--expected-object-store-format-version VALUE`
- `--expected-id-scheme VALUE`
- `--expected-digest-algorithm VALUE`
- `--expected-canonical-json-profile VALUE`

The command still creates the Store through `Engine::init` and then reads
`Engine::store_info`. Passing checks append field-specific
`*_match_expected=true` markers. Text and integer mismatches return
`QueryInvalid`.

## Consequences

Store creation can be used as a self-checking local setup step. Expectations are
evaluated after the Store is initialized; a mismatch reports failed acceptance
for the created Store rather than changing initialization rollback semantics.
This does not alter Store bootstrap, schema installation, manifest format, or
integrity validation.

## Implementation Findings

Adding more expectation fields pushed the already-large generated CLI command
enum over the default test-thread stack budget during full workspace tests. The
CLI now stores the top-level subcommand on the heap with `Box<Command>`. This
keeps the public CLI shape unchanged while reducing stack pressure from parsed
command values.
