# ADR-0080: Phase 3BN CLI Branch Record Queries

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  implemented Record/query slices.

## Context

Record and Record relation read APIs are commit-relative, which is the correct
Engine boundary. In CLI use, a local Agent often wants the current branch-head
view and otherwise has to run a separate branch-head command before every
`record show`, `record list`, `record relation-show`, or
`record relation-list` command.

## Decision

1. Keep the Engine APIs commit-relative.
2. Extend the four read-only Record CLI commands to accept exactly one of
   `--commit <id>` or `--branch <id>`.
3. When `--branch` is provided, the CLI resolves the active branch head through
   `Engine::branch_head` and then calls the existing commit-relative API.
4. This slice does not change mutation commands, WorkState replay, branch
   semantics, or output shape other than accepting branch-head targets.

## Consequences

- Local Agent workflows can inspect current Records and Record relations with
  one command per query.
- Core authority remains unchanged: storage and history logic still operate on
  explicit commit ids.

## Implementation Findings

- The existing CLI already used this branch-or-commit pattern for history and
  why. Record queries only needed the same target resolution pattern.
