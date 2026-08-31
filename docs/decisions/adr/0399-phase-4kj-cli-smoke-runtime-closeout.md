# ADR-0399: Phase 4KJ CLI Smoke Runtime Closeout

Status: Accepted
Date: 2026-08-31

## Context

ADR-0397 added a repository-level CLI smoke workflow, and ADR-0398 extended it
through verified Task completion. That process-boundary workflow now proves the
local tool loop can create and verify work, select the next runnable Task, and
complete the Task after the mandatory Acceptance Criterion gate is satisfied.

The workflow still leaves the active runtime coordination objects open: the
Claim created by `workvcs next` remains active, and the Session remains active
after the Task is completed. ADR-0390 and ADR-0391 already added script-
verifiable expectations for `workvcs claim release` and `workvcs session end`,
so the smoke workflow can prove the current CLI closeout path without adding
new Engine, Store, schema, runtime, or CLI semantics.

## Decision

Phase 4KJ extends `scripts/smoke-v0.1-cli-workflow.sh` to close the runtime
coordination state after verified Task completion.

The smoke workflow now proves:

- the Claim returned by `workvcs next` can be released through
  `workvcs claim release`;
- `claim release` reports the expected Session and `released` lifecycle state;
- the active Session can be ended through `workvcs session end` after the Claim
  release;
- `session end` reports the expected Session and `ended` lifecycle state.

This slice does not change Engine behavior, Store behavior, schema,
Verification closure, Task lifecycle semantics, Claim lifecycle semantics,
Session lifecycle semantics, or `next` selection. It only makes the already
implemented CLI closeout path part of the process-level smoke workflow.

## Consequences

The repository smoke workflow now exercises the local CLI loop from Store
bootstrap through verified Task completion and runtime closeout. Future smoke
extensions should keep this same boundary: they may prove already implemented
end-to-end CLI behavior, but semantic changes belong in focused Engine tests
and accepted ADRs.
