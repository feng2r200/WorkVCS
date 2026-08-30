# ADR-0384: Phase 4JU Store Info Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs store info` exposes the Store identity and manifest baseline through
the Engine boundary. Automation can read those fields, but acceptance scripts
still had to parse and compare them externally.

## Decision

The CLI adds optional expectations to `workvcs store info`:

- `--expected-store-id STORE_ID`
- `--expected-display-name NAME`
- `--expected-created-at-us VALUE`
- `--expected-store-format-version VALUE`
- `--expected-schema-version VALUE`
- `--expected-object-store-format-version VALUE`
- `--expected-id-scheme VALUE`
- `--expected-digest-algorithm VALUE`
- `--expected-canonical-json-profile VALUE`

The command still uses `Engine::store_info`. Passing checks append
field-specific `*_match_expected=true` markers. Store id expectations are parsed
as canonical Store UUIDv7 ids. Text and integer mismatches return
`QueryInvalid`.

## Consequences

Store manifest baselines become directly script-verifiable without direct SQL
or external comparison code. This does not add manifest mutation, change Store
bootstrap, alter integrity validation, or introduce a JSON output mode.
