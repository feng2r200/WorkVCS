# ADR-0346: Phase 4II Session Show Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs session show` exposes a session's lifecycle state, active workspace,
active branch, and focus. Automation needs to assert those fields directly
before running branch-local or focus-sensitive operations.

## Decision

`workvcs session show` accepts optional `--expected-lifecycle-state VALUE`,
`--expected-active-workspace WORKSPACE`, `--expected-active-branch BRANCH`, and
`--expected-focus ENTITY_OR_NONE`.

The CLI reads the session through the Engine facade and appends
`lifecycle_state_matches_expected=true`,
`active_workspace_matches_expected=true`, `active_branch_matches_expected=true`,
or `focus_matches_expected=true` for matching supplied expectations. Mismatches
return `SessionInvalid`.

## Consequences

Session-aware scripts can fail fast when their runtime context is not the one
they intend to operate on. The command does not change session start, switch,
focus update, end, list filtering, or claim semantics.
