# ADR-0410: Phase 4KU CLI Smoke Verification Requirement Closure

Status: Accepted
Date: 2026-08-31

## Context

ADR-0397 added a repository-level CLI smoke workflow for the current local
V0.1 tool surface. ADR-0398 then made the completion gate prove that a mandatory
Acceptance Criterion cannot complete until a passed Verification makes the
criterion effectively verified.

That path covers the zero-Verification-Requirement case. V1 also requires
stable AC-local Verification Requirement identity and requires Verifications to
target Requirements when an Acceptance Criterion has them. The core and CLI
already expose Verification Requirement creation, listing, and
Requirement-targeted Verification, but the repository smoke workflow does not
yet exercise that full operator path.

## Decision

Phase 4KU extends `scripts/smoke-v0.1-cli-workflow.sh` with a separate
VR-backed verification closure gate.

The smoke workflow creates a second Task with a required Acceptance Criterion,
creates one Verification Requirement under that criterion, records a passed
Verification targeted at the Requirement, verifies that the Requirement-targeted
Verification is discoverable through `verification list`, checks the owning
Acceptance Criterion projects to `verified`, and then transitions the Task to
`done`.

This is repository-level acceptance coverage over already implemented V1
semantics. It does not change Engine behavior, schema, Acceptance Criterion or
Verification Requirement state machines, Verification closure rules, Resource
applicability, command spelling, runtime coordination, Merge, Bundle, or any
other smoke gate.

## Consequences

The main smoke workflow now demonstrates both confirmed AC closure paths:

- direct Acceptance Criterion Verification when the criterion has no
  Requirements;
- Requirement-targeted Verification when the criterion owns stable Verification
  Requirements.

The final local CLI loop is more representative of V1 verification semantics
without expanding runtime behavior.
