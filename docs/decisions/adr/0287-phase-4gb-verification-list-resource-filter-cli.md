# ADR-0287: Phase 4GB Verification List Resource Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

Resource-backed Verification snapshots record the Resource basis used when
judging an Acceptance Criterion or Verification Requirement. The CLI can render
the number of resource basis entries and can show each entry, but list queries
could not answer which Verifications depend on a known Resource.

## Decision

`workvcs verification list` accepts `--resource <RESOURCE_ID>`.

The CLI parses the supplied value as a typed `ResourceId`, loads Verification
snapshots through the existing Branch/Commit query path, filters snapshots that
contain the Resource id in their recorded resource basis, and then applies any
`--limit`.

## Consequences

Users can trace Resource-backed Verification coverage from the command line
without bypassing the Engine facade or adding schema/API surface.

This slice does not change Verification recording, resource observation
semantics, applicability cache behavior, effective projection, Engine query
APIs, or schema.
