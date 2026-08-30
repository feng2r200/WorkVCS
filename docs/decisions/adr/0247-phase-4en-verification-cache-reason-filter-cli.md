# ADR-0247: Phase 4EN Verification Cache Reason Filter CLI

Status: Accepted
Date: 2026-08-30

## Context

ADR-0246 added branch-level `workvcs verification cache-list` output with each
cache row's applicability and reason code. Applicability has a closed
vocabulary and can be filtered through the Engine list options, while
`reason_code` is an exact recorded cache explanation string.

Operators need to narrow cache inspection by the recorded reason without
changing cache computation or adding a new semantic vocabulary.

## Decision

`workvcs verification cache-list` accepts optional
`--reason-code <REASON_CODE>`.

The filter is CLI-side exact matching over the Engine-returned cache list. It
can be combined with `--verification` and `--applicability`; all supplied
filters must match.

## Consequences

CLI users can inspect cache rows for one recorded applicability reason without
post-processing the full branch list.

This slice does not validate `reason_code` as a closed vocabulary, does not
change the Engine cache-list contract, does not recompute applicability, and
does not execute resource adapters.
