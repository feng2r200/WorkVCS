# ADR-0452: Phase 4MK Per-Code Recovery Guide

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LZ made WorkVCS business errors script-readable with stable key-value
stderr. Phase 4MJ extended the same shape to top-level clap syntax errors. The
V1 readiness ledger still had a recovery gap: operators could branch on stable
fields, but the repository did not define per-code recovery actions for the
current error taxonomy.

## Decision

Add `docs/operator/error-recovery-guide.md` as the current V1-local per-code
operator recovery guide. The guide covers every current core
`ErrorCode::as_str()` value, its current `error_category`, its current
`retryable` semantics, and the operator action to take before rerunning.

The guide also covers the CLI-local `cli_parse_error` introduced in Phase 4MJ.
It explicitly states that `message` is display-only context and that automation
should route by `error_code`, `error_category`, `retryable`, and, for usage
errors, `clap_error_kind`.

## Non-Goals

- No Rust code change.
- No new error codes, categories, or retryability semantics.
- No JSON error output.
- No command spelling redesign.
- No automatic repair workflow.

## Evidence

- Targeted taxonomy inventory from `crates/workvcs-core/src/error.rs`.
- Targeted CLI parse-error inventory from `crates/workvcs-cli/src/main.rs`.
- Docs-only validation and independent review are recorded in
  `docs/provenance/phase-4mk-per-code-recovery-guide.md`.

## Consequences

The current CLI error surface is now operator-actionable without requiring
operators to infer recovery behavior from raw messages or source code. JSON
output remains an Open V1 implementation contract item.
