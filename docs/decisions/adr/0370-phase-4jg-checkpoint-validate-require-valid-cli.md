# ADR-0370: Phase 4JG Checkpoint Validate Require-Valid CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs checkpoint validate` exposes the existing checkpoint validation result
with a `valid` field and problem text. Acceptance scripts still had to parse the
result to turn invalid checkpoints into command failure.

## Decision

The CLI adds `workvcs checkpoint validate --require-valid`.

The command continues to call the existing checkpoint validation path. When the
flag is supplied, an invalid result returns `QueryInvalid`; a valid result keeps
the same output and appends `valid_required=true`.

## Consequences

Checkpoint validation can be used directly as a failing acceptance step. This
does not add checkpoint validation rules, change checkpoint storage, change
checkpoint creation, or alter the schema.
