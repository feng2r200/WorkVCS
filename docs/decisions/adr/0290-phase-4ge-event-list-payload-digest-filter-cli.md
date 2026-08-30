# ADR-0290: Phase 4GE Event List Payload Digest Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

Event snapshots already expose canonical payload digests and payload JSON in
`event show` and `event list`. `event list` could filter by event kind, but not
by the payload digest that identifies the event content.

## Decision

`workvcs event list` accepts `--payload-digest <DIGEST>`.

The CLI parses the supplied digest as WorkVCS lowercase hex `Digest`, loads
events through the existing Event query path, filters by payload digest, and
then applies any `--limit`. When `--kind` or `--payload-digest` are present, the
limit is applied after CLI-side filtering so filtered results are not lost by an
early query limit.

## Consequences

Users can locate events by canonical payload content without changing Event
storage, schema, or Engine query APIs.
