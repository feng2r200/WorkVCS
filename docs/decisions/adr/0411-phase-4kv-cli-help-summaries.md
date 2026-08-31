# ADR-0411: Phase 4KV CLI Help Summaries

Status: Accepted
Date: 2026-08-31

## Context

The WorkVCS V0.1 CLI now exposes the local bootstrap, query, history, runtime,
verification, bundle, checkpoint, merge, and support surfaces from one
`workvcs` command. The current top-level help lists those commands, but most
business command descriptions are blank. That makes the command surface harder
to discover even though the command names themselves are stable.

This is an operator usability gap in the thin CLI layer. It is not an Engine,
schema, Runtime, WorkState, or business-semantics gap.

## Decision

Phase 4KV adds one-line summaries to the top-level `workvcs --help` command
list for the implemented command surface.

The summaries are clap command metadata only. They describe the command family
at a high level, keep command spelling unchanged, and do not introduce new
flags, outputs, transitions, persistence behavior, runtime coordination,
verification closure rules, Bundle behavior, Checkpoint behavior, Merge
behavior, scheduling behavior, or smoke workflow behavior.

A focused CLI regression test must guard that representative command summaries
are non-empty and that the old blank-description help shape does not return.

## Consequences

Operators can scan `workvcs --help` to identify the relevant command family
before entering a subcommand.

The implementation remains a thin command/input/output layer over existing core
APIs, and the slice does not change any V0.1 domain or persistence semantics.
