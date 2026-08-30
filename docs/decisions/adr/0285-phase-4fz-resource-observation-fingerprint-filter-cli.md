# ADR-0285: Phase 4FZ Resource Observation Fingerprint Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

ResourceObservation rows capture adapter-produced fingerprints and list output
already renders those fingerprints. Users can filter observations by resource,
adapter kind, adapter schema version, source Session, and detail content
metadata, but cannot directly find observations by a known fingerprint.

## Decision

`workvcs resource observation-list` accepts `--fingerprint <DIGEST>`.

The CLI parses the supplied fingerprint with existing lowercase hex digest
rules, lists observations through the current Engine API, filters returned
snapshots by exact fingerprint match, and then applies any detail filters and
`--limit`.

## Consequences

Users can discover ResourceObservations by adapter fingerprint without bypassing
the Engine facade or changing list output shape.

This slice does not execute adapters, compare drift, change verification
resource basis semantics, add new Engine query APIs, or change schema.
