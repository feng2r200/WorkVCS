# ADR-0300: Phase 4GO Typed ID Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

ADR-0215 added `workvcs id new --kind KIND` so manual workflows can mint
canonical typed UUIDv7 ids. Manual scripts also need a store-independent way to
validate ids received from logs, documents, or other CLI output.

## Decision

`workvcs id validate --kind KIND --id ID` validates the supplied id through the
same strongly typed canonical parser used by core and store-backed CLI
commands.

The command supports the same id kinds as `id new`, returns `valid=true` for a
successful parse, and rejects unknown kinds instead of falling back to an
untyped UUID parser.

## Consequences

CLI users can check typed canonical id text without opening a Store or writing
Rust code. The command does not reserve ids, write SQLite state, or introduce a
new identity scheme.
