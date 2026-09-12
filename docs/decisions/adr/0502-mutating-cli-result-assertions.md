# ADR-0502: Mutating CLI Result Assertions

Status: Accepted
Date: 2026-09-12

## Context

Several CLI commands accept `--expected-*` values and check them only after an
Engine operation has committed Runtime, Work-State, immutable provenance, or
filesystem output. A mismatch then exits non-zero using the same ordinary
query/runtime errors used before a write. The caller cannot safely tell whether
the operation failed or completed before the assertion failed, so a blind retry
can duplicate or conflict with already-applied work.

The earlier `session end` correction proved the narrow form of this problem:
its session identity and terminal lifecycle expectation are deterministic from
the command input and therefore belong before the write. The wider CLI also has
dynamic result assertions whose values are known only after an atomic Engine
operation, such as generated Claim identities, selected runnable Tasks, merge
outcomes, and imported object counts.

## Decision

Mutating CLI assertions use two explicit classes:

1. **Preflight assertions.** If an expected value is fully determined by CLI
   input or a stable command contract, parse and validate it before opening the
   writable operation. A mismatch must leave no command effect. `init`,
   `session start`, and `session end` use this class. Filesystem export/extract
   commands continue checking their available digest/count expectations before
   replacing or creating output.
2. **Committed-result assertions.** If the expected value depends on the
   operation's selected/generated/committed result, run the atomic Engine
   operation once and then validate it. A mismatch returns the distinct
   `mutation_postcondition_failed` error with `operation_completed=true`, the
   operation name, the complete rendered operation result, and guidance to
   inspect that result before any retry.

Core guards such as expected Branch head, expected state digest, manifest
identity, and idempotency remain inside their atomic Engine transitions. They
are not postconditions and retain their existing conflict semantics.

The bounded Wave A audit explicitly moves input- or contract-determined fields
for top-level `verify`, Claim transfer, merge resolve/freeze, and Handoff consume
into preflight. Generated Claim identities, selected Tasks, merge membership or
counts, and other operation-selected values remain committed-result assertions.

All mutating command handlers that expose result expectations must either
preflight them or route failures through the committed-result error. Read-only
query expectations keep their ordinary domain/query errors.

## Consequences

- A non-zero mutating command no longer leaves the caller unable to distinguish
  a rejected precondition from a completed operation with an unexpected result.
- Deterministic mismatches avoid writes entirely.
- Dynamic result assertions remain useful without pretending they are rollback
  guards; callers receive the IDs and state needed for recovery.
- Existing successful output remains unchanged. Error consumers gain one stable
  error code and explicit recovery fields.
