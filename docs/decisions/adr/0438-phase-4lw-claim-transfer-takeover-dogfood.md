# ADR-0438: Phase 4LW Claim Transfer and Takeover Dogfood

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LV proved one read-only another-project WorkVCS loop. The next
dogfood-biased ledger item was Claim transfer and takeover in realistic
continuation, because those paths were implemented and smoke-proven but still
too easy to treat as isolated CLI mechanics.

Phase 4LW keeps the same safety boundary as Phase 4LV: the external
`agent_soul` project is a read-only target, and all WorkVCS Store/log writes
stay inside the WorkVCS worktree.

## Decision

Accept a two-scenario dogfood loop for Claim handoff and recovery:

- Scenario A models cooperative continuation: one active Session owns a Claim,
  transfers it to another active Session, the receiver proves guard success,
  records verification evidence, and closes the Task.
- Scenario B models blocked continuation: another active Session is blocked by
  an existing exclusive Claim, reads the stable stale-takeover hint fields,
  observes that takeover from an active previous owner is rejected, marks that
  owner `potentially_stale`, performs explicit `claim takeover --force` with
  rationale, proves guard recovery, records verification evidence, and closes
  the Task.

The loop must keep the target project unchanged and must not add new display
fields unless the dogfood run exposes a concrete workflow blocker.

## Evidence

The successful dogfood run passed through the real CLI:

```text
phase4lw_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lw-claim-transfer-takeover-dogfood/.work-governance/runtime/dogfood/phase-4lw-20260901T020331Z.sqlite
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lw-claim-transfer-takeover-dogfood/.work-governance/runtime/logs/phase-4lw/claim-transfer-takeover.20260901T020331Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
transfer_previous_claim_id=01a05ab5-1d6d-7d32-8edb-27f360ba2998
transfer_claim_id=01a05ab5-1d7f-7470-ae01-3c4616d6f4d0
takeover_previous_claim_id=01a05ab5-1ecb-7bf0-8537-e3b3aafbd4a7
takeover_hint_claim_id=01a05ab5-1ecb-7bf0-8537-e3b3aafbd4a7
takeover_hint_previous_session_id=01a05ab5-1ea5-7633-9fc5-561da82e9f80
takeover_claim_id=01a05ab5-1f19-7231-b09d-acea23299864
target_status_unchanged=PASS
```

The first attempt failed when it tried to save a ContextPacket from an ended
Session:

```text
session invalid: session 01a05ab2-f422-7910-8523-5633727f06e7 is not active
```

That is not an implementation blocker. It confirms the existing runtime
boundary: context resolution is active-Session-scoped. The passing run starts a
new active continuation Session after Goal closeout before inspecting the
transition ContextPacket.

## Consequences

Claim transfer and stale-gated takeover are now dogfood-proven in a bounded
real external-project continuation loop. The proof remains local, read-only,
and operator-driven; it does not prove automatic stale detection, remote lock
coordination, adapter-backed re-observation, shared-mode maturity, or broad
multi-project operation.

Future slices should move toward Resource path/glob normalization or
adapter-backed re-observation when continuation workflows require it, and
broader `why` explanations before expanding display-only output.
