# ADR-0279: Phase 4FT History Limit Validation CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs history` exposes replayable commit ancestry from a Branch or Commit
starting point and already accepts `--limit <N>` through the Engine history
query options. Other bounded CLI list and inspection commands reject zero
limits at the user-entry boundary before doing store work, which makes invalid
usage deterministic and cheap to diagnose.

## Decision

`workvcs history` rejects `--limit 0` in the CLI before opening the target
store.

Positive limits continue to be passed to `HistoryQueryOptions` and retain the
existing history traversal and rendering semantics.

## Consequences

History output remains bounded by the existing Engine query path while the CLI
now reports the same zero-limit validation behavior as the other bounded tool
commands.

This slice does not change commit history traversal, branch state, replay,
Engine history APIs, or schema.
