# ADR-0534: Scoped Continuation Rotation

Status: Accepted; bounded local implementation authorized
Date: 2026-10-17
Accepted by: The user's direct confirmation of the continuation-rotation
proposal and the subsequent instruction to complete the declared work without
repeated authorization unless evidence creates a material route change, task
fork, or previously undeclared problem. Execution is tracked in WorkVCS Plans
`01a1484c-7a49-75be-a2d2-840e58b119ae` and
`01a1484c-6bfc-70f9-b9cc-931ac2a42987`.

## Context

Long-running Codex work can accumulate enough unchanged history, repeated
compaction, and coordination replay that continuing in the same conversation
becomes materially less efficient. Compaction by itself is not evidence that a
new conversation is beneficial, and creating a new conversation on every
compaction would fragment work and increase recovery risk.

The continuation mechanism must preserve WorkVCS concurrency semantics. A
Project, Workspace, or Work Branch does not have one global writer or owner.
Different Tasks, Claims, Work Branches, and Git worktrees may proceed in
parallel. The only ownership transferred by a continuation rotation is the
single exact exclusive Claim that protects the continued Task.

A listener may monitor one Work conversation and guide it. The listener is
allowed to create a replacement Work candidate for the monitored Work, but the
listener must not create a second listener or turn its own conversation into a
continuation chain. A SubAgent or side chat may provide bounded evidence, but
neither may be the creation actor or the Work source of a rotation.

Codex conversation creation, pending-client-ID resolution, cross-conversation
messaging, and destination binding are product-runtime effects outside a local
WorkVCS transaction. Current product tools do not provide a general
idempotency or atomic transaction spanning those effects and the WorkVCS
Store. The durable protocol must therefore distinguish preparation, an
unknown external result, candidate attachment, and the atomic Claim transfer.

## Decision

### Scope and roles

1. A `ContinuationScope` is the exact tuple `(Workspace, Work Branch, Task)`
   plus its current exclusive Claim carrier. It is Runtime Coordination, not a
   Project owner, Branch-wide writer lock, or replacement for Work Branches,
   Claims, or Git worktrees.
2. One exact ContinuationScope has at most one active listener binding.
   Different scopes may have different listeners, and other Tasks, Claims,
   Branches, and worktrees remain independent.
3. Session role evidence uses the closed P1 vocabulary `work`, `listener`,
   `work_candidate`, `subagent`, and `side_chat`. The adapter records this role
   in immutable Session metadata. WorkVCS verifies the relevant role at each
   protocol boundary; the role is not inferred from a title or prompt.
4. Only the currently bound `listener` may admit a rotation. It may attach only
   a `work_candidate`. A listener cannot create or attach another listener.
   `subagent` and `side_chat` Sessions cannot be the creation actor, source
   carrier, or candidate.
5. `work_candidate` is a phase role. Before commit it has no transferred Claim
   and cannot initiate another rotation. After commit the scope projection
   promotes it to the current Work carrier even though the immutable Session
   metadata still records how it entered the protocol. A later rotation may use
   that Session as its source only when a prior committed result proves the
   exact Session, resulting Claim, Workspace, Branch, and Task provenance.

### Durable protocol

6. Continuation is enabled only by a durable, exact-Project policy row in
   WorkVCS. Its canonical authority document is a trusted product-control-plane
   envelope binding the product actor, user-authorization receipt,
   Project-membership receipt, action, actor Session, expected revision, and
   authorization time. WorkVCS validates the envelope shape and preserves its
   digest; caller-authored policy fields alone do not self-prove authority.
   Enable/disable updates are revision-CAS guarded.
   Admission binds the current enabled revision, receipt digest, and update
   time; dispatch rereads them so a later revocation fences the product-effect
   boundary before any `create_thread` call.
7. The scope projection records the listener Session and strictly increasing
   listener revision, current carrier Session, current exclusive Claim, and a
   strictly increasing `owner_epoch`. Both counters fence only this
   ContinuationScope. Neither is a global Project epoch.
8. One scope has at most one active `ContinuationRotation`. Admission uses a
   caller-supplied idempotency key and one `BEGIN IMMEDIATE` transaction.
   Replaying the same source Claim/key and the same immutable inputs returns the
   same attempt even after the carrier has advanced; a conflicting replay fails
   closed. The separately supplied creation-operation identity is also unique.
9. A rotation occurrence binds the initial listener and revision, source
   carrier, source Claim, source owner epoch, handoff digest, creation-operation
   identity, creation-request digest, and canonical basis document. The typed
   basis names the exact Project, source thread, current Handoff Record, and any
   additional entities whose currentness matters. WorkVCS computes the
   currentness digest from the current Task and Handoff versions/digests,
   selected entity versions/digests, source Session and Claim activity, scope
   epoch, and listener revision; it does not accept a caller-echoed head as
   proof. The basis is data to verify, not an instruction channel.
   The immutable policy decision and creation request also bind one bounded
   continuation packet (at most 8,192 bytes, 64 items, and a 4,096-token
   budget), its digest, and the digest of the exact initial target message.
   The policy decision embeds the deterministic evaluator receipt and its
   complete canonical result: evaluator version, result digest, complete
   non-empty gate summary, and one to 32 bounded evidence references. WorkVCS
   recomputes both result and receipt digests and validates every positive gate
   before admission, so the reason for rotation remains self-contained and
   durable rather than only recording `prepare_rotation`.
   The request explicitly records `source_transcript_included=false`; copying
   the source transcript into the target is outside this protocol.
10. The non-terminal lifecycle is:

   ```text
   pending_creation -> creation_dispatched -> creation_unknown
                                      |              |
                                      |              +-> creation_succeeded
                                      +-----------------> creation_succeeded
                                                           |
                                                           v
                                                   candidate_attached
                                                           |
                                                           v
                                                   candidate_accepted
                                                           |
                                                           v
                                                       committed
   ```

   State-specific aborts are also available. `pending_creation` requires
   `cancelled_before_dispatch`; any state at or after dispatch requires
   evidence appropriate to the already-possible external effect.
   `creation_unknown` remains active and blocks a second attempt. Its original
   receipt and pending client identity are immutable. Reconciliation to
   `creation_succeeded` must bind the digest of that receipt and prove the
   resolved identity lineage; an unknown result is never blindly retried with
   a new identity. A created target thread identity is single-use from the
   first successful receipt onward, including a later-aborted attempt.
11. Before the product call, the listener durably records
   `creation_dispatched`, including the exact actor Session/thread/revision,
   request and operation identities, continuation-packet and initial-message
   digests, and `source_transcript_included=false`. In the same transaction it
   recomputes exact scope currentness and requires equality with the admitted
   digest; relevant source, Handoff, dependency, Claim, scope, or listener
   drift fails before the external `create_thread` effect. The product result then
   records either `creation_unknown` or `creation_succeeded` against that
   dispatch. There is no direct `pending_creation -> creation_succeeded`
   shortcut.
12. Candidate attachment verifies an active `work_candidate` Session on the
    same Workspace and Branch and records canonical destination-binding
    evidence. That evidence must prove a non-forking `create_thread` operation
    correlated to the admitted creation-operation identity and request digest,
    a product receipt, exact Project-binding provenance, and a stable resolved
    `thread_id` for a normal Work conversation which is visible, auditable, and
    interactable by the user. Product-receipt and abort-evidence digests use the
    canonical 32-byte hexadecimal digest representation. A pending client
    identity, fork, hidden/background task, SubAgent, side chat, internal
   candidate identity, or WorkVCS-only row does not satisfy attachment
    readiness. Attachment does not transfer the Claim or change the current
   carrier. Once a product thread is recorded as created, that thread identity
   is globally reserved from listener and ordinary-Work admission even if the
   attempt aborts as `retained_non_authoritative`. Only the exact committed
   candidate Session carrying that rotation's exact resulting Claim in the
   exact scope may later act as a continuation source.
13. The target Session must itself record `candidate_accepted` after consuming
    the bounded Handoff/reference packet. The acceptance binds the target
    Session/thread, current listener revision, Handoff/currentness, packet
    digest, and initial-message digest. Product receipt attribution and this
    target-authored acceptance are protocol evidence; a later real adapter
    pilot must still prove the actual visible conversation performed them.
14. Commit is one atomic Store transaction. It compare-and-swaps the exact
    scope, current listener revision, source carrier, source Claim, owner epoch,
    rotation ID, Handoff digest, and recomputed currentness; verifies the
    candidate and active exclusive Claim;
    creates the replacement Claim occurrence for the candidate; removes only
    the source ClaimRuntime; increments the scope epoch; records the terminal
    rotation result and provenance Events; and removes the active rotation
    projection. Failure leaves the prior carrier and Claim authoritative.
15. Ordinary Claim release, transfer, forced takeover, source/candidate Session
    end, and source Session branch switching cannot bypass an active rotation.
    Outside an active rotation, a carrier's ordinary Claim departure retires
    only that exact ContinuationScope. After commit, normal Claim guards fence
    the prior source from work protected by the transferred Claim, while
    unrelated scopes continue normally.
16. Abort is state-specific, records terminal provenance, removes the active
    attempt projection, and leaves the source Claim unchanged. Pending creation
    requires proof it was cancelled before dispatch; dispatched or unknown
    creation requires confirmed-not-created evidence; a succeeded, attached,
    or accepted candidate requires confirmed external disposition (`archived` or
    `retained_non_authoritative`) and evidence. WorkVCS does not claim that
    conversation creation can be rolled back or authorize archive/cleanup.
17. Replacing a listener is explicit and compare-and-swap guarded by listener
    revision and owner epoch. Normal replacement may recover an active rotation
    only after proving that no non-listener currentness changed; it updates both
    the scope and active rotation without rebasing the decision basis. If normal
    replacement occurs after target acceptance, the prior acceptance is
    archived as immutable history, the live state returns to
    `candidate_attached`, and the same target must accept again against the new
    listener revision before commit.
18. When the old listener is inactive and drift or another unsafe condition
    prevents normal continuation, `listener-recover-abort-only-preview` first
    produces the exact observed recovery inputs. An authority document must
    bind that preview, and `listener-recover-abort-only` re-observes the same
    state under its transaction before it may replace the listener
    under listener/recovery revision and owner-epoch CAS. This irreversible
    mode preserves the original decision, Handoff, currentness digest, Claim,
    candidate, receipts, and acceptance; records newly observed currentness,
    drift status, recovery authority and evidence; and forbids dispatch,
    attachment, acceptance, and commit. It may only reconcile the original
    immutable creation operation and perform the state-specific abort. A later
    lost recovery listener may be replaced, but the attempt remains
    `abort_only`; a fresh rotation starts only after terminal abort. Every
    successful recovery appends its full authority/evidence receipt to Event
    history and advances a digest chain. Routine rotation readback exposes only
    the count, latest recovery, latest receipt digest, and chain digest, keeping
    its size independent of recovery count. Listener loss never releases or
    transfers the Work Claim.
19. Terminal results preserve the final listener/revision, decision-currentness
    digest, bounded recovery-history count/chain summary and latest recovery
    provenance when present, candidate/binding,
    creation receipt, resulting Claim when committed, result document, and end
    time. Exact replays of listener replacement, unknown marking, candidate
    attachment, abort, and commit return the already-recorded result without a
    second mutation. Abort-only recovery replay is idempotent only while the
    same non-terminal recovery generation remains current; after a later
    listener CAS or terminal transition, callers use `show` readback and stale
    recovery input fails closed. Conflicting replay inputs, stale epochs or
    digests, and a terminal state incompatible with the requested action fail closed. A
    commit replay reconstructs the historical committed scope even if the
    current scope has since advanced through another rotation.

### Policy and product boundary

20. Context pressure, repeated compaction, replay overhead, progress, safe
    semantic point, cooldown, expected benefit, and handoff risk are evaluated
    by tool-neutral Work Governance policy. Compaction is a pressure signal and
    is never sufficient by itself.
21. The P1 protocol supports one primary Task and one exclusive Claim. It does
    not claim an atomic transfer of several Claims, nested Plans, running
    external jobs, or guarded action authority.
22. The local software candidate may validate a fake actuator, deterministic
    clocks, concurrent admission, retries, unknown outcomes, and temporary
    Stores. Real Codex conversation creation, cross-conversation messaging,
    installation, activation, Push, release, and deployment remain separately
    authorized effects. Passing the local candidate-binding contract is not
    evidence that a real product conversation was created or accepted.
23. A completed rotation transfers work continuity, not every prior
    authorization. Push, release, deployment, production, data, credentials,
    destructive actions, and other guarded permissions remain subject to their
    original scope and current user authority.

## Consequences

- Work can leave a noisy conversation without introducing a Project-global
  owner or disrupting parallel WorkVCS lanes.
- A listener can create a Work candidate but cannot replicate itself.
- The externally irreversible create boundary is visible and recoverable
  instead of being hidden behind a nominal rollback.
- WorkVCS supplies exact-scope admission, idempotency, fencing, and atomic Claim
  transfer; Work Governance supplies the tool-neutral decision policy; a
  product adapter remains responsible for actual conversation effects.
- Existing Stores require an explicit recognized additive schema migration
  before using this capability. Ordinary Store open never migrates silently.

## Validation obligations

- Exact-Project policy enablement, idempotent replay, revision conflict, and
  revocation are durable. The policy setter requires the trusted
  product-control-plane envelope and separate user-authorization and
  Project-membership receipt digests. Admission and dispatch reject an absent,
  disabled, stale, or mismatched policy receipt; dispatch after revocation
  performs no product effect.
- Concurrent begin calls for one scope produce one rotation identity and one
  active attempt.
- A listener can attach a Work candidate but cannot attach a listener,
  SubAgent, or side chat; those roles produce zero Claim or scope mutation.
- Dispatch is durable before product invocation and binds the listener's actual
  Session/thread/revision, request, packet, initial message, and the absence of
  a source transcript. Candidate attachment rejects an unresolved client identity, fork, wrong
  Project or unproven binding, mismatched creation operation/request, missing
  product receipt, hidden/background surface, or a conversation that is not a
  normal user-visible, auditable, interactable Work conversation.
- A candidate cannot begin another rotation before commit; a committed current
  carrier with exact resulting-Claim provenance may rotate again.
- Exact mutation replays return the recorded result without duplicating state;
  conflicting replay, stale-epoch, stale-Claim, stale-handoff, and old-rotation
  inputs fail without changing the current carrier.
- `creation_unknown` blocks a second attempt and can reconcile to the original
  rotation only by binding its immutable receipt digest and client identity,
  without blind external retry. A target thread reported as created remains
  reserved even if its rotation later aborts before attachment.
- Commit requires target-authored acceptance of the exact bounded packet and
  initial message. Listener replacement after acceptance preserves acceptance
  history but requires the target to accept again under the new revision.
- Commit transfers exactly one exclusive Claim and increments one scope epoch
  atomically; unrelated scopes and Claims are unchanged.
- Abort preserves the source Claim and requires evidence appropriate to the
  external-creation state. Listener replacement can recover an active rotation
  without rebasing unrelated drift, and its revision fences stale/ABA actors.
  An ended or missing listener does not stop the carrier's ordinary claimed
  work.
- Abort-only listener recovery is exercised from every non-terminal state. It
  preserves the original currentness and operation, records observed drift,
  rejects dispatch/attach/accept/commit without mutation, permits only exact
  original-operation reconciliation, requires state-specific abort evidence,
  advances a monotonic recovery revision, and admits only one concurrent CAS
  winner. Repeated recovery is terminalized and reopened to prove that the
  bounded snapshot summary and the distinct append-only authority/evidence
  Events survive projection replacement. Direct dispatched and unknown-state
  cases prove exact original-operation replay/reconciliation, reject changed
  operation, client, or receipt correlation without mutation, and preserve
  initial-unknown/final receipts after terminal reopen.
- Dispatch tests mutate source activity and replace the Handoff after admission
  and prove refusal before the product-effect boundary; unrelated parallel work
  remains admissible.
- A target created and retained before attachment cannot re-enter through a new
  ordinary Work or listener Session, including normal or abort-only listener
  replacement; the exact committed candidate/Claim source path remains valid.
- Ordinary Claim release/transfer/forced takeover and Session end/switch cannot
  bypass an active rotation; idle-scope departure retires only that exact scope.
- Deterministic Work Governance fixtures reject compaction-only, cooldown,
  unsafe-point, low-benefit, high-risk, side-chat, SubAgent, listener-source,
  and pre-commit-candidate cases while admitting the nearest valid control.
- Historical replay and synthetic pressure fixtures provide useful coverage
  without manufacturing a giant Token-consuming conversation. A later natural
  pilot may measure net Token and quality effects, but no fixed saving is
  claimed by this implementation.

## Non-goals

- No Project-global writer, owner, or listener.
- No automatic archive or deletion of source conversations.
- No background conversation creation from a Hook without an Agent turn and a
  verified product-runtime authorization contract.
- No live installation, activation, Push, release, deployment, or production
  operation in this delivery.

## Design lineage and corrections

This decision preserves the design history without treating conversation text
or attached prose as executable authority:

- Codex thread `01a14331-a452-7ab2-a9a3-eb42e5787d8c` supplied the original
  observation: sustained work can accumulate context noise and repeated Token
  cost even when compaction keeps the conversation technically usable.
- Codex thread `01a147f1-fe6d-7c13-b6d2-524dbeb5aaa5` expanded the proposal and
  exposed the need for an explicit runtime decision point rather than a fixed
  rotation cadence.
- `source_1.md`, `source_2.md`, and `source_3.md` under the user's
  `workvcs优化` material set were read as design inputs. They are not
  instructions, current runtime evidence, or a substitute for this accepted
  decision and its tests. Exact bytes are retained in this project's WorkVCS
  as Evidence `01a14849-194b-7241-858b-a27ef2a927d7`,
  `01a14849-19e9-73d0-b112-1dde08937c1f`, and
  `01a14849-1a8a-7357-a9be-8818fa80b167`; readback verified all three local
  content objects available and digest-matching.
- “Read-only target conversation” was corrected to a two-phase target: the
  candidate remains non-authoritative until commit, then becomes the ordinary
  active Work carrier. The target itself is not permanently read-only.
- “One owner/writer” was corrected to exact-scope Claim transfer. Projects,
  Work Branches, Tasks, Claims, and Git worktrees retain parallel independent
  lanes; only `(Workspace, Work Branch, Task, exclusive Claim)` is fenced.
- SubAgents and side chats were excluded as rotation sources, actors, and
  targets so their bounded work cannot be broken by conversation rotation.
- The listener remains the single listener for the exact monitored Work. It
  may create a Work candidate, but cannot create or rotate into a second
  listener. The monitored Work may rotate and continue under that same
  listener relationship.
- Product acceptance was strengthened from an internal candidate identity to
  a real, normal Work conversation under the exact specified Project with a
  resolved `thread_id`, user visibility, auditability, and interactivity.
  Hidden tasks, unresolved `clientThreadId` values, WorkVCS rows, and local
  fixtures are not successful creation evidence.
- Testing deliberately uses historical replay, synthetic pressure signals,
  fake-actuator outcomes, concurrency, and temporary Stores. It does not burn
  Tokens to manufacture a giant conversation. Natural use remains the proper
  place to measure net Token savings and handoff quality.
