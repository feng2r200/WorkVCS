# ADR-0281: Phase 4FV Query Limit Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

Most bounded CLI query tools now reject `--limit 0` before opening the target
store. Two remaining query surfaces still delegated zero-limit handling to
Engine option constructors:

- `workvcs store knowledge-exposure-list`
- `workvcs bundle import-list`

That left their user-facing error timing and wording inconsistent with the
rest of the CLI.

## Decision

The CLI rejects `--limit 0` before opening the target store for
`workvcs store knowledge-exposure-list` and `workvcs bundle import-list`.

Positive limits continue through the existing Engine query options and keep the
current filtering, ordering, rendering, and storage behavior.

## Consequences

The remaining bounded query tools now share the same fast-fail zero-limit
behavior as the rest of the CLI list and inspection surface.

This slice does not change Knowledge Exposure lifecycle, Bundle import attempt
semantics, Engine query APIs, or schema.
