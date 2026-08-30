# ADR-0217: Phase 4DJ Store Integrity CLI

Status: Accepted

Date: 2026-08-30

## Context

`Engine::validate_integrity` already provides Store integrity validation and
returns a structured `IntegrityReport`. `workvcs doctor` combines this check
with Store manifest reporting, but scripts and manual operators need a narrower
tool command that exposes only the integrity counters.

## Decision

1. Add `workvcs store integrity STORE`.
2. Reuse `Engine::validate_integrity`.
3. Render every `IntegrityReport` counter as stable key-value output.
4. Keep `workvcs doctor` unchanged as the combined manifest and integrity
   smoke-check command.

## Non-Goals

- This slice does not change Store validation behavior.
- This slice does not add new integrity rules.
- This slice does not alter Store bootstrap, migrations, or replay.
- This slice does not add JSON output mode.

## Consequences

- Local scripts can run integrity validation without parsing `doctor` metadata.
- Store integrity evidence remains available through the Engine boundary.
- The command keeps CLI growth aligned with already implemented core capability.
