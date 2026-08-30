# ADR-0354: Phase 4IQ Workspace List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs workspace list` renders Workspaces after optional display-name and
limit filters. Automation needs to assert the resulting Workspace count
directly.

## Decision

`workvcs workspace list` accepts optional `--expected-workspaces COUNT`.

The CLI applies existing filters, renders the same Workspace list, and appends
`workspaces_match_expected=true` when the rendered count equals the supplied
expectation. A mismatch returns `QueryInvalid`.

## Consequences

Workspace bootstrap scripts can fail fast when the visible Workspace set is
unexpected. The command does not change Workspace creation, Workspace show,
Branch creation, Branch heads, or list filtering semantics.
