# ADR-0252: Phase 4ES Show-At Branch CLI

Status: Accepted
Date: 2026-08-30

## Context

ADR-0013 added `show-at` by Commit as part of the first query/history vertical
slice. Many later CLI commands accept either a Commit or a Branch head selector,
but `show-at` still required callers to discover the Branch head Commit before
rendering the current WorkState.

## Decision

`workvcs show-at` accepts exactly one target selector:

- `--commit <COMMIT_ID>`
- `--branch <BRANCH_ID>`

When `--branch` is supplied, the CLI resolves the current Branch head through
the Engine facade and then calls the existing `show_at` replay path for that
Commit.

## Consequences

Users can render the current WorkState for a Branch without a separate branch
head lookup.

This slice remains read-only. It does not change replay semantics, does not
refresh branch projections, and does not add direct SQL access to the CLI.
