# ADR-0349: Phase 4IL Session List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs session list` renders sessions after optional lifecycle, workspace,
branch, focus, and limit filters. Automation needs to assert the resulting
session count directly.

## Decision

`workvcs session list` accepts optional `--expected-sessions COUNT`.

The CLI applies existing filters, renders the same session list, and appends
`sessions_match_expected=true` when the rendered count equals the supplied
expectation. A mismatch returns `QueryInvalid`.

## Consequences

Session-aware scripts can fail fast when their filtered session set is
unexpected. The command does not change session creation, switching, focus
updates, ending, claim behavior, or list filtering semantics.
