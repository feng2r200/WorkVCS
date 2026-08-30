# ADR-0207: Phase 4CZ Session Focus CLI

Status: Accepted

Date: 2026-08-30

## Context

The runtime Engine facade already supports setting and clearing Session focus.
The CLI could set focus indirectly during `session switch`, but it had no
standalone focus commands. This left users unable to adjust focus for an active
session without switching branches.

## Decision

1. Add `workvcs session focus-set STORE --session SESSION --focus ENTITY`.
2. Add `workvcs session focus-clear STORE --session SESSION`.
3. Use the existing `Engine::set_session_focus` and
   `Engine::clear_session_focus` APIs.
4. Render focus update results with session id, timestamp, lifecycle, focus id,
   and focus path count.
5. Keep all focus validation inside the existing runtime Engine path.

## Non-Goals

- This slice does not add CLI input for explicit focus paths.
- This slice does not change `session switch` focus behavior.
- This slice does not alter runnable or claim projections.
- This slice does not add new runtime tables.

## Consequences

- CLI users can adjust active session focus without switching branch context.
- `session show` can now validate focus changes directly.
- Explicit focus-path editing remains a later incremental tool slice.
