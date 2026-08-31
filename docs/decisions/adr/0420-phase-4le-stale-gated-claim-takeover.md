# ADR-0420: Phase 4LE Stale-Gated Claim Takeover

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LB added explicit Claim transfer and forced takeover. Phase 4LD then
added the missing `potentially_stale` Session state, but takeover still did not
require that state. That left recovery dependent on operator rationale alone.

The 4LE dogfood run also exposed an operator-guide mismatch:
`context --profile handoff` was documented, but the current CLI profile
vocabulary is only `brief`, `normal`, and `full`.

## Decision

Forced Claim takeover remains an explicit override and still requires:

```text
workvcs claim takeover STORE --session TAKING_SESSION --claim CLAIM --force --rationale TEXT
```

The operation now additionally requires the previous owning Session for the
active Claim to have lifecycle state `potentially_stale`.

The operation:

1. rejects takeover without `--force`;
2. rejects an empty rationale;
3. rejects active, ended, unknown, or invalid previous owning Sessions unless
   the previous owning Session is explicitly `potentially_stale`;
4. preserves the existing Claim replacement shape: the previous Claim is
   released, a new Claim occurrence is created for the taking Session, and the
   WorkState/Branch head remain unchanged;
5. records `previous_session_lifecycle_state` in the Core result, CLI output,
   and `claim.force_taken_over` Event payload; and
6. does not add automatic stale detection, schema migration, timeout policy, or
   remote coordination.

The operator guide should only document currently valid Context profiles until
a dedicated handoff profile is implemented.

## Non-Goals

- No automatic stale detection, heartbeat daemon, or timeout threshold.
- No takeover without explicit `--force`.
- No takeover from a merely active previous owning Session.
- No schema migration or new Claim lifecycle table.
- No Agent orchestration or distributed locking.

## Consequences

Recovery now has a two-step local proof chain: mark the previous Session
`potentially_stale`, then force-take over the active Claim with rationale. This
reduces accidental overwrites while keeping operator recovery possible.

The remaining V1 gap is dogfooding the recovery path in a fuller implementation
handoff and improving command ergonomics so operators do not have to manually
copy key-value IDs between steps.
