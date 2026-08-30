# ADR-0223: Phase 4DP Resource Observation Source Session CLI

Status: Accepted

Date: 2026-08-30

## Context

The Resource Observation core model already supports optional
`source_session_id` provenance. The CLI could record Evidence source sessions,
but Resource Observation recording did not expose the same provenance field.

## Decision

1. Add `--source-session SESSION` to `workvcs resource observe`.
2. Parse the value as a typed `SessionId`.
3. Reuse `ResourceObservationCreateOptions::with_source_session_id`.
4. Keep Resource Observation show/list rendering unchanged, because both already
   include `source_session_id`.

## Non-Goals

- This slice does not change Session lifecycle rules.
- This slice does not infer a current session implicitly.
- This slice does not add adapter execution.
- This slice does not change Evidence source-session behavior.

## Consequences

- Resource Observation provenance can identify the WorkVCS Session that captured
  it.
- Resource-backed verification workflows have stronger auditability through the
  existing CLI.
- The new parameter remains an explicit operator choice.
