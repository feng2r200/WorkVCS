# ADR-0351: Phase 4IN History Expected Entries CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs history` renders commit history after optional changeset, commit kind,
operation, state digest, and limit filters. Automation needs to assert the
resulting number of history entries directly.

## Decision

`workvcs history` accepts optional `--expected-entries COUNT`.

The CLI applies existing filters, renders the same history output, and appends
`entries_match_expected=true` when the rendered entry count equals the supplied
expectation. A mismatch returns `QueryInvalid`.

## Consequences

History validation scripts can fail fast when a filtered history query returns
an unexpected number of entries. The command does not change history traversal,
filtering, show-at behavior, replay, commit creation, or storage semantics.
