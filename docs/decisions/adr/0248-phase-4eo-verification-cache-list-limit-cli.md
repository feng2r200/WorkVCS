# ADR-0248: Phase 4EO Verification Cache List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs verification cache-list` can enumerate branch-level verification
applicability cache rows and filter by verification, applicability, and reason
code. A branch may contain many cached verification rows, so CLI users need a
bounded way to inspect a prefix of the list.

The list remains a read-only inspection surface over recorded cache rows.

## Decision

`workvcs verification cache-list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered cache list
after all other supplied filters have been applied. The ordering remains the
Engine list ordering from ADR-0246.

## Consequences

CLI users can inspect bounded cache-list output during local tool workflows.

This slice does not change the Engine cache-list contract, does not add schema,
does not recompute applicability, and does not execute resource adapters.
