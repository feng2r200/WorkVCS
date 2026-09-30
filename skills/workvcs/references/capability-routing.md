# Capability routing

WorkVCS is a versioned work-and-knowledge system, not only a note store. Choose
the smallest capability set that preserves the state the current work actually
needs. Required participation does not mean every capability must be used.

## Scenario map

| Situation | Prefer | Durable result |
| --- | --- | --- |
| Start or resume project work | `project discover`, then bounded `recall` or `resume` | Correct ProjectRef ownership and the smallest current recovery packet |
| Small task with durable meaning but no coordination need | `capture` | Atomic Finding, Decision, Question, Risk, Attempt, Evidence, or Knowledge without a Plan |
| Existing stage baseline but no WorkVCS history | Baseline bootstrap in `workflows.md` | Provenance-labelled reconstructed state followed by live WorkVCS authority |
| Dependent phases, multi-turn work, or costly recovery | `plan admit` or `plan evolve`, with Goal/Plan/Task only as needed | Versioned work graph and explicit next work without a mega-Plan |
| Completion has real conditions | Acceptance Criteria, Verification Requirements, `verify`, `verification`, and `evidence` | Inspectable acceptance contract, result, and supporting material |
| Validation depends on files, repositories, or other observable inputs | `resource`, observations, and applicability refresh | Evidence tied to an exact observed basis and explicit drift state |
| Several Agents or Sessions share execution | `session`, focus, `claim`, `next`, and `runnable` | Explicit coordination without rewriting Work State |
| Work pauses or changes owner | focused `handoff`, optionally a saved `context-packet` | Bounded continuation state with stable identifiers |
| A prior conclusion may now be stale | `record currentness-audit`, then guarded transition or correction | Explicit currentness decision without inventing changes |
| Need to explain or reconstruct history | `why`, `history`, `diff`, `show-at`, `commit`, `changeset`, and `event` | Causal, structural, or historical evidence at a named state |
| Explore an alternative route | Work Branch plus Attempt; later `merge`, `restore`, or abandon | Alternative state and outcome remain inspectable without contaminating the main branch |
| Need portable or recoverable state | `checkpoint`, `bundle`, `doctor`, and Store inspection | Verified recovery or interchange boundary; no remote sync is implied |
| One result concerns several projects | `capture --capture-group`, one primary delivery, immutable secondary references | One mutable semantic authority with discoverable cross-project association |
| Need to record permission evidence | authorization receipt commands | A redacted mechanical claim; never permission for the actual action |

## Match structure to value

- Use standalone `capture` for meaningful cognition when a Plan or execution
  graph would add no coordination value.
- Use Goal/Plan/Task when dependencies, ownership, acceptance, sequencing, or
  recovery benefit from explicit structure. Do not manufacture entities merely
  to show that WorkVCS participated.
- Use Session and Claim state only when runtime coordination is real. A record
  or capture does not require an active Session.
- Use Evidence for immutable supporting material and Verification for a
  judgment about a requirement. A command log alone is not the judgment.
- Use Resource observations when the truth of a result depends on a changing
  source. Mechanical drift does not itself rewrite a semantic conclusion.
- Use Knowledge only for a reusable conclusion with scope and provenance, not
  as a duplicate of every Finding.

## Record at semantic boundaries

Under required participation, make a narrow write after a durable semantic
change and before moving far enough that recovery would lose the causal link.
Typical checkpoints are a confirmed scope, an accepted decision, a route-
changing failed Attempt, a verified Finding, new Evidence, a Task transition,
a blocker, or a handoff. Several file edits or test commands that support one
unchanged conclusion should normally become one coherent capture.

Use one writer per semantic slice. Share stable IDs or a focused Handoff with
other workers instead of copying broad context or creating competing mutable
records.

## Do not claim absent capabilities

WorkVCS does not currently parse transcripts, infer semantic records, perform
embedding search, orchestrate Agents, watch Resources in the background,
synchronize to a cloud service, or automatically activate guarded control-
plane mutations. The calling Agent supplies meaning explicitly and retains the
authorization boundary.
