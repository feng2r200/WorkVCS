# ADR-0535: Session-selected work context and recovery

Status: Accepted for the user-approved local implementation
Date: 2026-10-26
Authority: The user approved the complete optimization proposal with “方案通过”
in conversation `01a17707-4775-7af3-9ae3-5bd0d8dae3b8`, after explicitly
confirming the five final-outcome requirements and clarifying flexible Branch
selection from a Project's conversation relationships.

## Decision

1. ProjectRef resolves ownership, Store, and Workspace. Its registered Branch
   is the default work target, not a requirement that every conversation use
   that Branch. Work Branch selection is independent of Git Branch identity.
   Related conversations may share a Branch; independent work may select or
   create another Branch; continuation may preserve or explicitly switch it.
2. Routed reads and operations accept an explicit existing Session or Branch.
   A Session supplies its current Workspace, Branch, and optional Focus. A
   separately supplied Branch must agree. The selected Branch must be active
   and belong to the resolved Workspace. Conflicting evidence fails closed.
3. Routed durable operations preserve that selection in an optional typed
   `work_context` on the immutable Capture intent. Existing intents without
   this field retain their exact serialized form and default-target semantics.
   Idempotency compares work context as well as semantic payload and group.
   Replaying an operation cannot silently move it to another Branch.
4. Project binding receipts continue proving the registered project binding.
   Delivery receipts prove the selected work target in that same Store and
   Workspace. Runtime Session drift before an undelivered operation is applied
   is reported explicitly. A committed receipt remains readable after a
   Session ends or switches; its target never changes.
5. Recovery selects applicable records before ranking or truncation. Exact
   Task/Focus links and explicit scope selectors identify current-work state.
   Shared project facts remain distinguishable from task instructions and
   authority. An unclassified historical record is reference evidence, not
   authorization or a current next action. No-Plan work can use record scope
   without creating a Plan. Explicit Task/Focus selectors must agree with the
   Session Focus. An explicit non-path work identity without an entity suppresses unrelated Task,
   acceptance, readiness, dependency, and Claim content; the original Session
   Focus remains runtime provenance. Context, Resume, Recall, and capture use
   the same derived applicability rule. A path-only selector preserves an
   explicitly selected Task or Session Focus; without either, it filters the
   Project overview. Paths alone never create a No-Plan work identity.
   Every Record/Knowledge item labels its
   applicability and grants no action authority by itself.
   Retrospective Evidence follows the same selection before ranking and budget.
   Routed capture and Plan operations attach the selector and verified origin
   Workspace/Branch to `evidence[*].metadata.work_scope`; contradictory fields
   fail before admission. Plan operations accept the same work-selection flags
   and tag their accompanying Records as well. Immutable capture/admission/
   evolution commits prove Evidence introduction and inheritance through the
   selected head's ancestry. An origin Branch is provenance, not a permanent
   fence against an evidenced descendant fork. A fork before introduction does
   not inherit that Evidence. Current typed Verification citations traverse
   the selected Task's AC/VR references with exact parent, local-key and semantic
   version checks. Unclassified Store-global Evidence remains available by
   exact EvidenceId and is not enumerated as Project/Branch context. A Session's
   current mutable Focus cannot prove historical Evidence ownership.
6. A recovery basis binds owner, selected Branch, task/scope, head, runtime
   identity, and item versions. Incremental recovery reports changes and
   removals against that basis; a changed owner or selection invalidates it.
   The exact supplied ancestor is reconstructed and its projected semantic
   versions must match the cached digest. Each response returns the current
   head's new basis; live runtime observations remain outside semantic deltas.
   The basis also binds selected Evidence identities, immutable introduction
   commits, and qualifying Task/AC/VR, Verification and relation versions.
   Unchanged Evidence is not replayed in a delta. A newly relevant standalone
   Evidence item whose prior state cannot be reconstructed from the supplied
   commit invalidates the cached basis and requires a fresh recovery.
7. Governance retains the complete final goal, achieved outcomes, remaining
   gaps, actual blockers, and current authority. Local test, review, commit,
   and handoff results do not substitute for the user's final outcome.

## Evidence and acceptance

The observed Buddy conversations both used the correct ProjectRef but recalled
the same default Branch and received records from the other task. Core Session
switching and Branch creation already exist. The missing connection is the
project-facing selection and recovery path, plus same-Branch applicability.

Acceptance covers shared and separate Branches, Session switches, continuation,
No-Plan scopes, unrelated-task exclusion, shared facts, conflicting selections,
idempotent delivery, changed Session state, exact receipt readback, and a final
goal that remains actionable after a local repair.

The design uses existing Session, Branch, Focus, Task, Claim, Record, and journal
semantics. It introduces no parallel work identity or permissions ledger.
OpenAI's [session memory guidance](https://developers.openai.com/cookbook/examples/agents_sdk/session_memory)
and Anthropic's [context engineering guidance](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
support task-relevant recovery and explicit external state; the concrete
routing contract follows the user's approved WorkVCS requirements.
