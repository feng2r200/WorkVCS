# ADR-0238: Phase 4EE Workspace List Display Name Filter CLI

Status: Accepted

Date: 2026-08-30

## Context

Workspace list can enumerate all Workspaces in a Store, but users need a simple
way to locate a Workspace by display name. The existing Workspace list output
already exposes `display_name`.

This slice adds a read-side CLI filter only. It does not change Workspace
creation, identity, initial Branch semantics, storage schema, or the Engine
workspace list API.

## Decision

1. Add `--display-name NAME` to `workvcs workspace list`.
2. Load Workspace snapshots through the existing Engine facade.
3. Apply an exact display-name filter before rendering when the option is
   supplied.
4. Keep matching case-sensitive and byte-for-byte with the stored display name.

## Non-Goals

- This slice does not add fuzzy search, prefix search, or case-folded matching.
- This slice does not add workspace lifecycle state.
- This slice does not change workspace identity or initial branch behavior.
- This slice does not change SQLite schema, replay, or core snapshot APIs.

## Consequences

- CLI users can locate a named Workspace without post-processing all rows.
- The command remains a thin read-only wrapper over existing Engine snapshots.
