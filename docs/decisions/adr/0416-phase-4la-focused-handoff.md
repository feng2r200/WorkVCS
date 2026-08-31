# ADR-0416: Phase 4LA Focused Handoff

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger now identifies Handoff as the next dogfood-biased gap.
WorkVCS already has two required ingredients:

- SessionEnd creates an immutable SessionDiff with a canonical summary; and
- `Record(kind=handoff)` creates an explicit versioned WorkState record.

However, the two surfaces are not connected ergonomically. A receiving Agent can
see that a Handoff record exists, but cannot read the linked SessionDiff summary
through a focused Handoff command.

## Decision

1. Add a read-only Engine API for SessionDiff snapshots.
2. Add focused `handoff create` and `handoff show` CLI commands.
3. `handoff create` remains an explicit WorkState operation. It creates a
   `Record(kind=handoff)` and writes a fixed scope object containing the source
   Session id, optional SessionDiff id, lifecycle state, and optional focus
   Entity id.
4. `handoff show` loads the Handoff record at a branch head or commit, validates
   that it is a Handoff record, parses the fixed scope object when present, and
   renders linked SessionDiff summary when a SessionDiff id is available.
5. Existing `record handoff`, `record show`, and `session end` remain supported
   for lower-level and diagnostic workflows.

## Non-Goals

- No automatic Handoff creation during SessionEnd.
- No transcript parsing, LLM-generated summary, or implicit handoff inference.
- No new schema, typed Handoff table, or background materialized view.
- No remote/cloud handoff, cross-Store synchronization, or Agent orchestration.
- No replacement of generic Record commands.

## Consequences

The local tool now supports a practical continuation loop: end a Session with a
summary, explicitly author a Handoff record that points at that SessionDiff, and
let a receiving Agent inspect the Handoff and linked SessionDiff without direct
SQLite access.

## Implementation Findings

- Existing Record scope is sufficient for a fixed Handoff wrapper payload in
  V1. A separate Handoff payload schema can remain deferred until more dogfood
  examples exist.
- `handoff create` needs explicit `--focus` when an already-ended Session has
  cleared runtime focus but the author still wants to carry a concrete
  continuation target.
- The first implementation proof is smoke-level. Durable dogfood should use the
  new command to hand off an actual implementation slice before promoting any
  typed Handoff relation or Context packet expansion.
