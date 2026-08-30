# ADR-0352: Phase 4IO Event List Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs event list` renders events after selecting exactly one target
dimension and applying optional kind, payload digest, and limit filters.
Automation needs to assert the resulting event count directly.

## Decision

`workvcs event list` accepts optional `--expected-events COUNT`.

The CLI applies existing filters, renders the same event list, and appends
`events_match_expected=true` when the rendered count equals the supplied
expectation. A mismatch returns `QueryInvalid`.

## Consequences

Event validation scripts can fail fast when a filtered event query returns an
unexpected number of events. The command does not change event recording, event
show, changeset lookup, history traversal, or list filtering semantics.
