# ADR-0212: Phase 4DE WorkState Diff CLI

Status: Accepted

Date: 2026-08-30

## Context

Core already exposes `Engine::diff` for comparing two WorkState targets, where
each target can be either a Commit or a Branch head. The CLI could show a full
state at one Commit and could restore historical WorkStates, but it lacked a
direct command for comparing two states and listing entity or relation version
changes.

## Decision

1. Add top-level `workvcs diff STORE`.
2. Accept exactly one source selector: `--from-commit` or `--from-branch`.
3. Accept exactly one destination selector: `--to-commit` or `--to-branch`.
4. Parse all ids through strongly typed canonical UUIDv7 parsers.
5. Reuse `Engine::diff` and render the resolved targets plus entity/relation
   added, removed, and updated version changes.

## Non-Goals

- This slice does not change WorkState diff semantics.
- This slice does not add semantic inference over entity or relation content.
- This slice does not refresh projections before reading a Branch head.
- This slice does not add JSON output or filtering.

## Consequences

- Users can inspect WorkState changes without writing Rust code.
- Branch-head and Commit targets are explicit in command input and output.
- Diff remains a read-only operation over existing replayed WorkState data.
