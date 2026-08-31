# Context Profile Budget Dogfood Evidence

Status: Phase 4KX local dogfood evidence
Recorded: 2026-08-31

This file records the first non-temporary local dogfood probe for the
item-budgeted `context` packet. The Store file is local runtime data and is
ignored by Git:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/runtime/dogfood/phase-4kx-context-profile-budget.sqlite
```

The probe used the local Phase 4KX CLI build to:

1. initialize a WorkVCS Store;
2. create one Workspace and one pending Task for the context-budget slice;
3. create one active Decision, one Finding, and one active Knowledge statement;
4. start an active Session on the Workspace Branch;
5. resolve default `context` to obtain the current state digest;
6. resolve `context --profile normal --budget-items 4` with that expected
   state digest.

Observed packet envelope:

```text
context_profile=normal
context_budget_items=4
context_available_items=8
context_items=4
context_omitted_items=4
context_omission_priorities=4
context_omission_categories=4
matches_expected=true
```

Observed retained item categories:

```text
context_item.0.category=session_anchor
context_item.1.category=branch_overview
context_item.2.category=current_task
context_item.3.category=task_readiness
```

Observed omission summary:

```text
context_omission_priority.0.priority=P4
context_omission_priority.0.omitted=1
context_omission_priority.1.priority=P6
context_omission_priority.1.omitted=1
context_omission_priority.2.priority=P7
context_omission_priority.2.omitted=1
context_omission_priority.3.priority=P8
context_omission_priority.3.omitted=1
context_omission_category.0.category=active_decision
context_omission_category.0.omitted=1
context_omission_category.1.category=finding
context_omission_category.1.omitted=1
context_omission_category.2.category=scoped_knowledge
context_omission_category.2.omitted=1
context_omission_category.3.category=session_continuity
context_omission_category.3.omitted=1
```

Post-review recheck:

After independent review, `ContextPacket` was tightened to expose a bounded
envelope instead of the full `ContextOverview`, and same-priority packet items
now preserve deterministic source order. The dogfood packet command above was
re-run against the same Store, Session, and expected state digest; the packet
still reported `context_profile=normal`, `context_budget_items=4`,
`context_items=4`, `context_omitted_items=4`, and `matches_expected=true`.

Residual gap:

This is dogfood proof for the profile and whole-item budget mechanism. It is
not proof that the full V1 Context Resolver is complete. Acceptance Criteria,
Goal/Plan path packets, richer blocker context, Attempt execution detail,
focused Handoff reading, path-sensitive Knowledge policy, packet persistence,
and Agent protocol encoding remain outside this slice.
