# Phase 5E Default-Stack CLI Validation

Status: current local evidence
Date: 2026-09-12

## Question

Can the complete WorkVCS test suite run through its ordinary documented command
without a caller-supplied 32 MiB `RUST_MIN_STACK`, and does splitting the large
CLI test source actually solve the observed failure?

The durable Finding is Record
`01a0961a-0a6e-7f81-95fd-7c856c1747be`.

## Diagnosis

Focused default-stack runs reproduced overflows in small and large Record
workflow tests. Serializing the harness with `--test-threads=1` did not help.
An experimental `Box<RecordCommand>` left the first focused failure unchanged,
as did a short-lived parse-and-run helper on the ordinary test thread.

This distinguishes per-test execution stack from source-file size: moving the
same test body into another module does not increase the 2 MiB Rust test-thread
stack. The existing `run_cli_test_with_large_stack` helper already expressed
the narrow boundary correctly for older parser-heavy workflows.

## Correction

Ten unwrapped full-parser workflows now delegate their bodies through that
existing 16 MiB isolated helper:

- Finding, Assumption, Decision, Decision lifecycle, Question/Risk, Attempt,
  and Handoff Record workflows;
- Record show and atomic Decision supersession; and
- the direct multi-change Assumption causal-detail workflow.

No production parser, command contract, Store schema, or runtime path changed.
The ineffective boxing and parse-helper experiments were removed before final
validation. Strict Clippy also exposed three pre-existing, behavior-neutral
test-expression lints: one redundant `into_iter` and two redundant formatting
borrows. Those expressions were simplified rather than adding new lint
exceptions.

## Validation

The following command passed with 200 CLI tests and no stack environment
override:

```bash
cargo test -p workvcs-cli --quiet
```

The complete default-stack workspace suite also passed. Formatting and diff
checks and Clippy with warnings denied passed at the repository's established
`too_many_arguments` boundary. Final evidence also includes required-valid
Store doctor and a packaged/global-install smoke test from the exact committed
source.

## Boundary

Historical Phase 4PE and ADR evidence keeps the old
`RUST_MIN_STACK=33554432` commands because they describe those earlier runs.
Current operator instructions no longer require them. A future physical split
of the CLI test module may still improve ownership and compile ergonomics, but
it is not represented as a stack fix without new evidence.
