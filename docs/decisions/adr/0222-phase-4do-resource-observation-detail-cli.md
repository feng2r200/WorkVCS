# ADR-0222: Phase 4DO Resource Observation Detail CLI

Status: Accepted

Date: 2026-08-30

## Context

The core Resource Observation model already supports optional detail-content
metadata through `ResourceObservationDetailInput`. The CLI could record
Observation fingerprints and summaries, but it could not attach detail-content
digest, size, media type, or format metadata.

## Decision

1. Add detail-content arguments to `workvcs resource observe`:
   `--detail-content`, `--detail-content-file`, `--detail-content-digest`,
   `--detail-content-size-bytes`, `--detail-media-type`, and
   `--detail-format-metadata-json`.
2. Keep detail content sources mutually exclusive.
3. For text or file input, compute digest and size through
   `ResourceObservationDetailInput::from_raw_bytes`.
4. For digest input, require `--detail-content-size-bytes` and use
   `ResourceObservationDetailInput::from_digest`.
5. Reuse existing Resource Observation show/list rendering.

## Non-Goals

- This slice does not add adapter execution.
- This slice does not add blob retrieval.
- This slice does not change Resource Observation fingerprint semantics.
- This slice does not add multiple detail content entries.

## Consequences

- Resource-backed verification workflows can persist detail-content metadata
  through the CLI.
- Observation detail metadata remains governed by the core Resource model.
- Operators can inspect the recorded detail metadata through existing
  `observation-show` output.
