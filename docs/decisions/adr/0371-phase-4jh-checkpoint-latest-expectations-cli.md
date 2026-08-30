# ADR-0371: Phase 4JH Checkpoint Latest Expectations CLI

Status: Accepted
Date: 2026-08-30

## Context

`workvcs checkpoint latest` reports whether a usable checkpoint exists for a
commit and, when found, renders its checkpoint id. Acceptance scripts still had
to parse `checkpoint_found` and `checkpoint_id` externally.

## Decision

The CLI adds two optional checks:

- `workvcs checkpoint latest --require-found`
- `workvcs checkpoint latest --expected-checkpoint CHECKPOINT`

The command continues to use the existing latest usable checkpoint query.
`--require-found` fails when no usable checkpoint exists. `--expected-checkpoint`
fails when the found checkpoint id does not equal the supplied id, including the
case where none is found. Passing checks append `checkpoint_found_required=true`
and `checkpoint_matches_expected=true` respectively.

## Consequences

Checkpoint recovery and bundle-checkpoint acceptance scripts can fail directly
on missing or unexpected latest checkpoints. This does not change checkpoint
selection, usability semantics, checkpoint validation, or storage schema.
