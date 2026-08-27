# Confirmed State v0.1 Provenance

This document preserves the detailed promotion chain for the initial WorkVCS
confirmed-design baseline. It is supporting evidence, not a second normative
product or architecture specification. Current design authority remains in the
documents linked from [`docs/README.md`](../README.md).

Architecture Specification decisions 81–188 later closed several questions
that were Open in this initial snapshot. Their promotion chain is recorded in
[Confirmed State v0.2 Provenance](confirmed-state-v0.2.md). The statements below
remain an accurate history of v0.1 and must not be read as overriding v0.2.

## Source and evidence boundary

The baseline was promoted on 2026-08-24 from confirmed decisions in the
ChatGPT conversation `WorkVCS需求沟通`
(`6a847cdb-7f00-83e8-8870-a8acee174a4c`) under the user's explicit request to
establish repository-native confirmed design state.

The initial two-page `read_thread` response is recorded as governance evidence
`5e9a4c2b3ce2df194f0d1f9fc015575577e5d870bdb2b6c1eca58688d8dd8338`
with source digest
`af47a2995530909d6043e48de3921cb9ebc2f51144795981c80aba56cc79d946`.
It contains the 17-turn core-decision and repository-handoff snapshot. Two
later delivery-instruction turns are
`c8e010b0-03f9-407a-894c-a4a22bb07cae` and
`03c09f68-d18f-4728-9563-09b74b087810`. The smaller promotion extract is
recorded as evidence
`5272d95afd881f20ecbcd3cc33b7eeb90aa725a5084c9325450cb314677e2c1e`
with source digest
`9e1e7bbf9d18b93734093d4c8429a57466ba9681663a1c3b1327dcfe5e0958e1`.

These records preserve connector-returned bytes, but the connector supplies no
cryptographic source attestation. Source authenticity is therefore degraded:
the records support reconstruction and consistency review but cannot by
themselves prove that caller-supplied content was not substituted. The
conversation remains provenance rather than normative authority.

## Confirmation ledger

| Decision range | Confirmed subject | User confirmation turn and response |
|---|---|---|
| Initial intent | Agent-facing Plan/Task/Decision-log CLI with Git-like branch and merge | `3c324463-d1ed-4091-b3f5-6544d07c3276` |
| Product boundary A–D | Work-State version control rather than a Todo CLI; Work Branch semantics independent of Git; structured Decision history rather than model chain of thought; the Agent calls the CLI and the CLI does not launch or orchestrate the Agent | `0e19656b-5e34-4cb1-b2ed-fecd9859550c`: `ABC对;D对，同时我基本不打算往另一个方向走` |
| 1–8 | Work Branch, history, merge, Session, AC, Decision granularity | `ce1af243-5bb0-45c9-96ff-311934a0e47b`: `1C 2D 3C 4C 5是 6是 7... 8V1是A` |
| 9–14 | Knowledge, Assumption, Attempt, optional/recommended AC with a Verification gate for automatic completion, Verification/Evidence, automatic WorkStateCommit | `00fd0410-9f92-469a-8a53-9609b4e15172`: `9是 10是 11是 12B且同意你的说法 13是 14自动commit` |
| 15–24 | Goal/Plan/Task structure, scoped Knowledge, branch/provenance ownership, merge semantics, Session/Branch, context | `d5cc8c37-9c38-47a6-9f91-0720c4901c22`: `15A; 16A; ... 21 认同 ... 24认同` |
| 25–28 | Multiple active Plans, recursive Plan/Task refinement, Plan revision, post-completion cognition | `36375b08-aff4-40e3-9ae9-caa0d15cbc23`: `25B；26需要Plan hierarchy ... 27 十分认可；28 十分认可` |
| 29–33 | Workspace-level Plan, independently executable parent Task after decomposition, Plan order, mixed children, terminal states | `82c25da7-98a2-423c-a07c-6dc789a56910`: `29是；30B; 31需要；32 允许混排；33必须区分` |
| 34–38 | Task status/outcome, late Goal discovery, explicit Plan/Goal completion, next semantics | `baa1221c-6e50-4c8a-a1c9-00338b340f73`: `34同意 35认可 36认可 37显式 38同意` |
| 39–46 | Persistent three-way merge, two parents, custom resolution, retained learning, Decision subject/scope, Branch isolation | `7feaebf8-ee24-4b6f-966a-dbac9946b6c9`: `39是 40是 41是 42C 43是 44认可 45认可 46认可` |
| 47–55 | Same-Branch concurrency, claims, takeover, focus, deterministic budgeted context, atomic claim-next | `7e04735b-fd94-4ebc-b29d-39adb326196a`: `47A 48是 ... 53是 54V1支持 55必须` |
| 56–65 | Runtime/provenance split, Branch-scoped claims, typed relations, rationale, stable AC IDs, atomic ChangeSet/batch | `52b8060f-8552-41b1-a0d8-5b37bf10905d`: `56是 57是 58C 59B ... 64是 65是且需要batch` |
| 66–75 | Focus path, scoped context, profiles/budget, causal exception, Session diff, explicit semantic records, Attempt lifecycle, Verification wrapper | `ea536664-eda1-4d1c-b966-86a43a2ec5f3`: `66认可 67是 ... 73是 74认可 75V1支持` |
| Engineering boundary | Monorepo/multi-Workspace and non-Git support, Agent adapters, Agent-readable storage, delegated identity choice, independent Git inclusion, low Git coupling with accurate drift basis, single-machine V1, and scale/history working-set concerns | `f037d3f7-c798-4d2f-99d0-8c54f523255c` |
| 76–80 | Store, Knowledge Space, Session Context Set, SQLite/object-store direction, non-destructive core history | `2d6194c5-e7a0-4f0f-83ef-26d6790169b7`: `五项判断的推荐选项均同意` |
| Product name | `WorkVCS` | `d3b05fb2-32ba-4ee8-9e39-503ed7a82f41`: `确认产品名为"WorkVCS"` |
| Baseline delivery | Generate a confirmed state for Codex, then establish the repository documentation baseline without runtime implementation | `c8e010b0-03f9-407a-894c-a4a22bb07cae`; `03c09f68-d18f-4728-9563-09b74b087810` |

The ledger records promotion provenance, not a parallel contract. A numbered
confirmation is interpreted against its exact question or accepted option;
surrounding assistant examples and recommendations do not become confirmed by
proximity.

## 2026-08-25 correction

The assistant review in turn `66b924a0-628a-4186-a541-45146633ccdb`
identified six bounded correction areas. The current repository task explicitly
requested applying those corrections and rechecking consistency. The exact
request sentence is bound by SHA-256
`b62fd84ce384920650a73269058cd92db08b025c7318091790334aec3898d80c`.
The assistant turn is the review source; the current user request is the
promotion authority. The resulting baseline therefore treats Question and Risk
as confirmed Record kinds, Verification as versioned semantic state with
immutable Evidence references, Knowledge Space exchange mechanics as Open,
Session provenance on WorkStateCommit as conditional, concrete evidence
persistence as unfixed, and this detailed ledger as supporting provenance
outside the hot documentation entry point.
