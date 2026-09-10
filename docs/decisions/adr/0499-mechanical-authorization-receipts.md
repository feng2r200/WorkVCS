# ADR-0499: Mechanical Authorization Receipts

Status: Accepted / P0-3a and P0-3b implemented
Date: 2026-09-10

## Context

Work-governance needs a durable mechanical record of which already-decided
authority applies to a bounded WorkVCS action. That record must not be confused
with a semantic Decision, an authorization policy judgment, or execution of an
external action. It must also remain safe to project into context and `why`
without exposing sensitive authority references.

P0-3a issue/show/list and P0-3b consume are implemented as mechanical receipt
surfaces. Revoke and receipt projection into `context`/`why` remain pending.

## Decision

An `AuthorizationReceipt` is a dedicated Record subtype. A Decision Record
must not be used as a receipt surrogate. The P0 command surface is:

```text
workvcs receipt issue  ...
workvcs receipt consume ...
workvcs receipt show   ...
workvcs receipt list   ...
```

`revoke` is explicitly deferred and not current.

Each receipt is bound to one Work Branch and carries:

- a target identity and target version/digest;
- the permitted action identity;
- the governing contract digest;
- an authority-ref type/kind, digest, and redacted marker; and
- expiry and lifecycle metadata sufficient to reject stale use.

The structured `authority_ref.ref` input is automatically redacted: its
original text is not persisted or emitted in scope, payload, CLI output,
`show`, `list`, debug output, or any other receipt projection. Persisted receipt
state retains only the authority-ref kind, digest, and redacted marker. This
boundary applies to `authority_ref.ref` only; it is not a full-manifest secret
scanner. `rationale` is a persisted field, so callers must not put credentials,
tokens, or other secrets in it.

Issue is a single transaction. It is CAS-guarded and idempotent: a successful
replay with the same operation identity returns the same mechanical result,
while a conflicting replay is rejected. The transaction either commits the
complete receipt binding or exposes no partial state.

Receipt issue rejects:

- target version or target digest drift;
- an expired receipt or contract;
- mismatched branch, target, action, contract digest, authority digest, or
  expected receipt version.

Consume is branch-scoped single-use. Its idempotency path may reuse only a
previously committed `workstate_commit` result for the same operation identity;
an orphan ChangeSet must never trigger `reused`. Consume does not promise a
Store-global lock across restore or branch histories. It is its own
single-transaction, CAS-guarded transition and is not atomic with any external
action.

WorkVCS records mechanical binding and lifecycle facts. It does not decide
whether authority is sufficient, valid under organizational policy, or adequate
for the requested action. WorkVCS also does not execute or atomically couple a
receipt transition to an external action; external execution and its policy
gates remain separate work-governance responsibilities.

## Projection boundary

P0-3a `show` and `list` project mechanical receipt state subject to the same
redaction boundary: receipt identity, branch/target binding, action, contract
digest, lifecycle, expiry, consumed state, authority-ref type, authority-ref
digest, and `authority_ref_redacted=true`. They never expose authority-ref
original text. Receipt projection into `context` or `why` is not implemented
and must not be claimed as current.

## Current capability boundary

P0-3a issue, show, and list plus P0-3b consume are current capabilities.
`revoke` and receipt projection into `context`/`why` are deferred and not
current. They are not prerequisites for the implemented P0-3a/P0-3b surfaces.

## Non-goals

- No Decision Record masquerading as an AuthorizationReceipt.
- No authorization sufficiency, policy, approval, or confirmation judgment in
  WorkVCS.
- No transaction spanning receipt state and an external action.
- No original `authority_ref.ref` text in durable receipt state or
  context/`why` projections; callers must keep credentials, tokens, and other
  secrets out of persisted `rationale` (there is no full-manifest secret scan).
- No `revoke` implementation in this P0 slice; it is deferred.

## Consequences

Governance can later prove the mechanical target/action/contract binding and
replay behavior without turning WorkVCS into an authorization engine. Drift,
expiry, and repeated consumption fail closed. The separate transaction boundary
keeps external action failures and policy decisions visible to the owning
workflow rather than hiding them behind a receipt mutation.
