# ADR-0418: Phase 4LC Operator Quickstart and Recovery Docs

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger identifies CLI discoverability and operator use as the
next dogfood-biased gap. The implementation now exposes the main local CLI
families, including Store, Workspace, Goal, Plan, Task, AC/VR, Verification,
Resource, Session, Claim, Context, Handoff, Merge, Checkpoint, and Bundle
commands. The repository also has process-level smoke coverage.

The remaining problem is practical use. A local operator or future Agent still
has to infer install, smoke, minimal workflow, handoff, and recovery commands
from tests and ADRs. That pushes the project toward a highly tested CLI kernel
without a clear day-to-day operating path.

## Decision

Add `docs/operator/quickstart-and-recovery.md` as the current V0.1 local
operator guide. The guide documents:

1. build and install options using the current Cargo workspace;
2. repository validation commands that operators should run before trusting a
   local build;
3. a minimal Store, Workspace, Session, Task, Claim, Context, Verification, and
   Handoff loop;
4. common recovery actions for stale verification caches, blocked Claims,
   ended Sessions, integrity failures, and smoke failures; and
5. explicit current boundaries and non-goals.

## Non-Goals

- No release packaging, Homebrew formula, shell completion, or installer.
- No daemon, server, remote sync, cloud collaboration, GUI, or TUI.
- No automated transcript parsing, LLM extraction, or Agent orchestration.
- No `potentially_stale` Session state or automatic stale Claim takeover.
- No claim that the current implementation is V1 complete or dogfood complete.

## Consequences

The next local user of WorkVCS has a single operator-oriented entrypoint instead
of reconstructing workflow commands from smoke scripts. This improves V1
readiness without expanding runtime behavior or narrowing the effort back to
expectation-only smoke edits.
