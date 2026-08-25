# WorkVCS Documentation

This directory is the repository-native source of truth for WorkVCS's
confirmed product, domain, and architecture state.

## Current baseline

The first confirmed baseline contains:

- [Product definition](product/product-definition.md)
- [V1 and V2 boundary](product/v1-v2-boundary.md)
- [Domain entities](domain/entities.md)
- [Typed relationships](domain/relationships.md)
- [Domain invariants](domain/invariants.md)
- [System boundaries](architecture/system-boundaries.md)
- [Versioning engine](architecture/versioning-engine.md)

These documents specify what WorkVCS currently means. They do not claim that
the described runtime has been implemented.

## Baseline provenance

This baseline was promoted on 2026-08-24 from the confirmed decisions in the
ChatGPT conversation `WorkVCS需求沟通`
(`6a847cdb-7f00-83e8-8870-a8acee174a4c`) under the user's explicit request to
establish the repository's confirmed design state. The conversation is
supporting provenance; the files linked above are the durable normative state.

The complete two-page `read_thread` response is recorded as governance
evidence
`5e9a4c2b3ce2df194f0d1f9fc015575577e5d870bdb2b6c1eca58688d8dd8338`
with source digest
`af47a2995530909d6043e48de3921cb9ebc2f51144795981c80aba56cc79d946`.
It contains all 17 source turns, including the original assistant questions
and options needed to reconstruct the numbered confirmation ranges. The
smaller promotion extract remains recorded as evidence
`5272d95afd881f20ecbcd3cc33b7eeb90aa725a5084c9325450cb314677e2c1e`
with source digest
`9e1e7bbf9d18b93734093d4c8429a57466ba9681663a1c3b1327dcfe5e0958e1`.

These current-run records preserve the bytes returned through the connector,
but the connector supplies no cryptographic source attestation. Their source
authenticity is therefore **degraded**, not independently verified: they
support reconstruction and consistency review but cannot by themselves prove
that the caller did not substitute content. This limitation does not elevate
the conversation to normative status or weaken the user's explicit promotion
request; it limits only the strength of the provenance-audit claim. Local
governance evidence may not travel with a Git clone, so the conversation and
turn IDs below remain the durable retrieval route.

### Confirmation ledger

The following ledger preserves the confirmation chain without making the chat
a second normative specification. Turn references are immutable conversation
turn IDs; quoted response fragments are the user's confirmations.

| Decision range | Confirmed subject | User confirmation turn and response |
|---|---|---|
| Initial intent | Agent-facing Plan/Task/Decision-log CLI with Git-like branch and merge | `3c324463-d1ed-4091-b3f5-6544d07c3276` |
| 1–8 | Work Branch, history, merge, Session, AC, Decision granularity | `ce1af243-5bb0-45c9-96ff-311934a0e47b`: `1C 2D 3C 4C 5是 6是 7... 8V1是A` |
| 9–14 | Knowledge, Assumption, Attempt, optional/recommended AC with a Verification gate for automatic completion, Verification/Evidence, automatic WorkStateCommit | `00fd0410-9f92-469a-8a53-9609b4e15172`: `9是 10是 11是 12B且同意你的说法 13是 14自动commit`; the accepted item 12 requires Verification for every mandatory AC before automatic `done` when AC exists |
| 15–24 | Goal/Plan/Task structure, scoped Knowledge, merge semantics, Session/Branch, context | `d5cc8c37-9c38-47a6-9f91-0720c4901c22`: `15A; 16A; ... 23 V1支持显式session switch ... 24认同` |
| 25–28 | Multiple active Plans, recursive Plan/Task refinement, Plan revision, post-completion cognition | `36375b08-aff4-40e3-9ae9-caa0d15cbc23`: `25B；26需要Plan hierarchy ... 27 十分认可；28 十分认可` |
| 29–33 | Workspace-level Plan, independently executable parent Task after decomposition, Plan order, mixed children, terminal states | `82c25da7-98a2-423c-a07c-6dc789a56910`: `29是；30B; 31需要；32 允许混排；33必须区分` |
| 34–38 | Task status/outcome, late Goal discovery, explicit Plan/Goal completion, next semantics | `baa1221c-6e50-4c8a-a1c9-00338b340f73`: `34同意 35认可 36认可 37显式 38同意` |
| 39–46 | Persistent three-way merge, two parents, custom resolution, retained learning, Decision subject/scope, Branch isolation | `7feaebf8-ee24-4b6f-966a-dbac9946b6c9`: `39是 40是 41是 42C 43是 44认可 45认可 46认可` |
| 47–55 | Same-Branch concurrency, claims, takeover, focus, deterministic budgeted context, atomic claim-next | `7e04735b-fd94-4ebc-b29d-39adb326196a`: `47A 48是 ... 53是 54V1支持 55必须` |
| 56–65 | Runtime/provenance split, Branch-scoped claims, typed relations, rationale, stable AC IDs, atomic ChangeSet/batch | `52b8060f-8552-41b1-a0d8-5b37bf10905d`: `56是 57是 58C 59B ... 64是 65是且需要batch` |
| 66–75 | Focus path, scoped context, profiles/budget, causal exception, Session diff, explicit records, Attempt and Verification wrappers | `ea536664-eda1-4d1c-b966-86a43a2ec5f3`: `66认可 67是 ... 73是 74认可 75V1支持` |
| Engineering boundary | Monorepo/multi-Workspace and non-Git support, Agent adapters, Agent-readable storage, delegated identity choice, independent Git inclusion, low Git coupling with accurate drift basis, single-machine V1, and scale/history working-set concerns | `f037d3f7-c798-4d2f-99d0-8c54f523255c`: `human-readable并非必须...Agent工具可以准确读取并准确理解`; `不要高度耦合...知道是否漂移并提供准确的依据`; `ID模型...选择最合适的方式` |
| 76–80 | Store, Knowledge Space, Session Context Set, SQLite/object-store direction, non-destructive core history | `2d6194c5-e7a0-4f0f-83ef-26d6790169b7`: `五项判断的推荐选项均同意` |
| Product name | `WorkVCS` | `d3b05fb2-32ba-4ee8-9e39-503ed7a82f41`: `确认产品名为"WorkVCS"` |
| Baseline delivery | Promote confirmed state into this repository; do not implement runtime code | Current user request, 2026-08-24 |

The detailed meanings consolidated from those confirmed ranges are the linked
product, domain, and architecture documents. The ledger records promotion
provenance subject to the degraded source-authenticity limitation above; it
does not create a parallel contract to maintain.

The engineering-boundary confirmation requires accurate Agent-readable output
and concise, actionable errors. It does not fix JSON encoding, schema or error
code catalogs, or process exit-code conventions.

### Confirmed-only promotion rule

Only a direct user requirement or the exact content of an explicitly accepted
option is eligible for this baseline. Assistant recommendations, examples,
candidate implementations, and details surrounding an accepted numbered item
remain proposals unless the user separately confirms them. A confirmation of
a numbered range is therefore interpreted against the corresponding questions,
not against every statement in the assistant response that contained them.

## Authority and conflict handling

For repository work, follow the authority order in [`AGENTS.md`](../AGENTS.md).
Within the documentation set, accepted ADRs explain and override earlier
architectural choices; domain invariants constrain architecture; architecture
specifies how the confirmed domain is realized; product documents define value
and release scope. Implementation notes must conform to all of them.

If two confirmed documents appear to conflict, stop treating either
interpretation as settled. Record the conflict, trace it to its confirming
evidence, and resolve it through an ADR or an explicit user decision before
implementation.

## State classification

- **Confirmed:** accepted product, domain, and architecture statements in this
  baseline.
- **Proposed:** alternatives or implementation candidates that still require a
  decision. Proposals are not allowed to redefine confirmed state.
- **Historical:** superseded decisions and prior baselines retained for
  provenance.
- **Derived:** indexes, summaries, generated views, and runtime projections.
  They may be regenerated and are not independent authority.

## Updating the baseline

A change to confirmed design must identify its source, update every affected
document, check the invariants, and receive validation at the same impact level.
Material architecture choices require an accepted ADR. Conversation history is
supporting evidence, not a substitute for this repository baseline.
