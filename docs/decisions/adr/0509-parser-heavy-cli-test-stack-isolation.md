# ADR-0509: Parser-Heavy CLI Test Stack Isolation

Status: Accepted
Date: 2026-09-12

## Context

The operator quickstart required
`RUST_MIN_STACK=33554432 cargo test --workspace --quiet` because several unit
tests parse and execute the complete Clap command tree on Rust test threads.
Those threads default to a substantially smaller stack than the CLI process's
main thread. New Record lifecycle tests reproduced the overflow with the
default stack; `--test-threads=1` did not change it.

Two tempting fixes did not match the evidence. Moving test source into more
modules or files would change compilation and ownership but not the stack
available to each test thread. Boxing `RecordCommand` also left the focused
overflow unchanged because Clap still constructs the complete nested command
definition during parsing.

The test module already had one narrow helper that runs parser-heavy bodies on
an isolated 16 MiB thread. Most older full-parser workflows used it, while ten
unwrapped Record and causal-query workflows did not.

## Decision

Use the existing isolated-thread helper for the ten empirically failing or
same-family full-parser test bodies. Keep assertions and test discovery on the
ordinary harness; only the parser-heavy body receives the larger local stack.

Remove the caller-facing 32 MiB `RUST_MIN_STACK` instruction after the complete
CLI and workspace suites pass without that environment variable. Historical
provenance retains commands that were true at the time they were recorded.

Do not split the large CLI source solely as a stack workaround. A dispatcher or
test-module split remains a separate maintainability decision and needs its own
evidence, ownership boundary, and regression plan.

## Consequences

- Developers and CI can run the documented workspace command without hidden
  environment setup.
- The larger stack is limited to known full-parser tests rather than applied to
  every Rust test thread in the process.
- The public CLI, parser, Store schema, and runtime behavior do not change.
- Newly added full-parser tests must use the isolated helper when a focused
  default-stack probe demonstrates the same condition; parser-free tests stay
  on the ordinary harness.

## Non-goals

- Redesigning the Clap command hierarchy.
- Splitting the 67k-line CLI module as an incidental side effect.
- Hiding arbitrary stack overflows by increasing a process-wide limit.
- Rewriting historical validation evidence.
