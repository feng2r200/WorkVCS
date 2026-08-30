# ADR-0215: Phase 4DH Typed ID CLI

Status: Accepted

Date: 2026-08-30

## Context

Phase 1 defined strongly typed UUIDv7 ids and canonical lowercase hyphenated
text form. Most Store-backed commands allocate ids internally, but manual
canonical and low-level tooling sometimes needs valid ids without writing Rust
code.

## Decision

1. Add top-level `workvcs id`.
2. Add `workvcs id new --kind KIND`.
3. Generate only existing public WorkVCS typed id kinds.
4. Return lowercase hyphenated UUIDv7 text through each typed id's Display
   implementation.
5. Reject unknown id kinds instead of producing an untyped UUID.

## Non-Goals

- This slice does not add arbitrary UUID parsing or conversion.
- This slice does not mint ids inside a Store.
- This slice does not reserve generated ids or write any SQLite state.
- This slice does not add non-UUID identity schemes.

## Consequences

- CLI users can generate valid typed ids for canonical and manual validation
  workflows.
- Generated ids remain type-specific at the command boundary.
- Store mutations continue to allocate authoritative ids internally.
