# ADR-0216: Phase 4DI Store Info CLI

Status: Accepted

Date: 2026-08-30

## Context

`workvcs doctor` already opened a Store, read its manifest, and ran integrity
validation. Scripts and manual operators also need a lighter command that only
reports the Store manifest baseline without performing a full integrity pass.

## Decision

1. Add `workvcs store info STORE`.
2. Reuse `Engine::store_info`.
3. Render Store identity, display name, creation timestamp, manifest version
   fields, id scheme, digest algorithm, canonical JSON profile, and canonical
   manifest JSON.
4. Keep `doctor` unchanged as the integrity-oriented command.

## Non-Goals

- This slice does not change Store bootstrap.
- This slice does not add manifest mutation.
- This slice does not replace `doctor`.
- This slice does not add JSON output mode.

## Consequences

- Scripts can inspect Store manifest metadata without running integrity checks.
- Store baseline data is available through the CLI without direct SQL access.
- `doctor` remains the higher-cost validation command.
