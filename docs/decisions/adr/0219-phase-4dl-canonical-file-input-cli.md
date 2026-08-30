# ADR-0219: Phase 4DL Canonical File Input CLI

Status: Accepted

Date: 2026-08-30

## Context

The canonical CLI already exposed Store-independent encoding and digest helpers
for inline JSON, text, and hex inputs. Manual operators and scripts also need to
run the same checks against files without embedding full payloads into command
arguments.

## Decision

1. Add `--json-file PATH` to `workvcs canonical encode`.
2. Add `--json-file PATH` to `workvcs canonical digest`.
3. Add `--content-file PATH` to `workvcs canonical content-digest`.
4. Keep each command's input source mutually exclusive.
5. Reuse the existing canonical parser and digest functions after reading file
   bytes.

## Non-Goals

- This slice does not add stdin streaming.
- This slice does not add JSON output mode.
- This slice does not change canonical encoding or digest rules.
- This slice does not read or mutate Store state.

## Consequences

- Conformance vectors and local payload files can be checked directly through
  the CLI.
- File-backed canonical checks stay under the same Phase 1 authority as inline
  inputs.
- Raw ContentObject digest calculation remains distinct from canonical semantic
  JSON hashing.
