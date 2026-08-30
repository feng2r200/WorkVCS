# ADR-0275: Phase 4FP Session List Limit CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs session list` exposes Session snapshots and already supports filtering
by lifecycle, Workspace, Branch, and current focus. Session output can grow as
tooling records more work sessions, so CLI users need bounded output for quick
inspection and scripted reads.

## Decision

`workvcs session list` accepts optional `--limit <N>`.

The CLI rejects `--limit 0`. Positive limits truncate the rendered Session list
after all supplied filters have been applied.

## Consequences

Users can bound Session discovery output without changing Session lifecycle,
focus state, context state, runnable projection, history state, or storage
semantics.

This slice does not change Session start, switch, focus, end, Engine Session
query APIs, or schema.
