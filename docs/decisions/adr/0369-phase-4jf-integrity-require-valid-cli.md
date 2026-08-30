# ADR-0369: Phase 4JF Integrity Require-Valid CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs doctor` and `workvcs store integrity` both expose the existing Engine
integrity report. Automation can already parse `invalid_checkpoints`, but a
direct require flag makes integrity checks usable as a single acceptance step.

## Decision

The CLI adds `--require-valid` to:

- `workvcs doctor`
- `workvcs store integrity`

Both commands continue to call `Engine::validate_integrity`. When
`--require-valid` is supplied, the command fails with `QueryInvalid` if the
returned integrity report contains any invalid checkpoints. Otherwise it appends
`valid_required=true` to the existing output.

## Consequences

Acceptance scripts can require a clean integrity report without parsing the
counter externally. This does not add new integrity rules, change checkpoint
validation, change doctor/store-integrity output fields, or alter storage
schema.
