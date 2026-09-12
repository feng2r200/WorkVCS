# Phase 5D Bounded Multi-Agent Context Bridge Evidence

Status: current bounded local evidence
Date: 2026-09-12

## Question

The pilot tested whether a fresh SubAgent can recover enough project truth from
WorkVCS to contribute without inheriting the parent conversation, and whether
increasing Recall from 20 to 40 items produces a justified reduction in
follow-up work. It also tested what can actually be attributed about token cost
before governance prompts are shortened.

The durable Question is Record
`01a09463-4e90-7730-a0e2-6ce00d77d25a`.

## Setup

Two independent Agents used `gpt-5.6-terra` at medium reasoning with no parent
turns inherited. They were forbidden to read the repository, project docs,
memory, or the parent conversation and could not write WorkVCS, Git, or files.
Both first read the installed WorkVCS Skill and then used the installed CLI
against the maintained project binding. Their matched delegation contracts
differed only in the requested `resume` item budget.

Visible fixed inputs were measured as bytes and words, not claimed as model
tokens:

| Input | Measured size |
| --- | ---: |
| Delegation prompt | 849 characters, 1,551 UTF-8 bytes, 61 whitespace-delimited units |
| Installed WorkVCS `SKILL.md` | 80 lines, 547 words, 3,980 bytes |
| Complete installed WorkVCS Skill tree | 274 lines, 1,607 words, 11,955 bytes |

## A/B observations

| Observation | Budget 20 | Budget 40 |
| --- | ---: | ---: |
| Available Recall items | 27 | 27 |
| Returned / omitted | 20 / 7 | 27 / 0 |
| Resume wall time | 2.27 s | 2.36 s |
| Captured output | 129 lines, 541 words, 11,594 bytes | 162 lines, 779 words, 15,190 bytes |
| Additional WorkVCS reads | 2 | 3 |

Both Agents recovered the active Goal, current Task, exclusive Claim, remaining
priority order, and the latest portability Finding. Neither could strictly
prove the exact terminal outcome of the preceding Task from generic bounded
Recall alone. Raising the budget returned seven more items and increased the
captured output by 3,596 bytes, but did not remove that ambiguity.

The result supports a delegation-contract rule rather than a larger default
budget: when an Agent must prove a predecessor's exact terminal outcome, pass
the stable Task, Evidence, artifact, or focused Handoff identity and query it
directly. Generic Recall remains a current-work projection and does not promise
that an arbitrary terminal Task will fit.

## Final blind recheck

After the corrections, a fresh read-only turn of the independent reviewer was
given the exact preceding and current Task ids. It again inherited no project
or conversation context. Across five WorkVCS reads, `resume --budget-items 20`
returned 20 of 29 available items and omitted nine; the Agent then used the
stable predecessor id for targeted closeout inspection and a complete brief
projection.

The Agent independently recovered that the preceding portability Task was
`done`, had one required AC and VR backed by a passed Verification and one
Evidence, and had an untruncated closeout with `gaps_total=0`. It recovered the
current in-progress Task and exclusive Claim, found no blocker, and placed the
default-stack CLI test work before installation ergonomics. A complete
53-item brief projection omitted the mitigated raw-Evidence Risk. The reviewer
therefore judged the handoff-relevant current state self-consistent while still
refusing to generalize the bounded Evidence case or assign platform token
percentages.

This closes the original handoff ambiguity without making generic Recall retain
all terminal Tasks: exact terminal proof came from an explicit stable identity
and a focused read.

## Independent challenge and corrections

A separate blind currentness review first found that an older pre-fix Finding
still appeared active beside its replacement. The parent corrected it through
the existing Finding lifecycle. A later blind review then found active Risk
`01a090e7-aded-7e42-bd5e-31c8a86c765d`, whose statement said raw Evidence was
not portable even though Bundle v2 had repaired and verified that exposure.

This was not a prompt-quality problem. Question and Risk had no terminal
lifecycle, so resolved unknowns and exposures could remain mechanically
current forever. Phase 5D therefore added guarded Question and Risk terminal
states and used the globally installed binary to move that Risk to
`mitigated`. Brief Recall now excludes the terminal state while retrospective
history preserves it.

The first terminal-state Bundle probe also exposed that the supported
same-Store apply allow-list excluded every `record` entity. A different-Store
probe correctly remained outside the local profile. A same-identity copied
Store probe, however, proved that even one missing Record commit was rejected.
The targeted correction admits the existing generic Record entity family while
leaving external-Store activation and missing-Branch creation out of scope.
Installed-binary dogfood then passed with:

- `bundle_payload_files=11` and `bundle_payload_references=27`;
- `preflight_action=same_store_fast_forward_ready`;
- `apply_outcome=same_store_fast_forward_applied`;
- four imported commits and four imported EntityVersions;
- imported Question status `answered` and Risk status `mitigated`; and
- required-valid target Store doctor.

The rejected probes are retained as diagnostic attempts, not behavioral proof:
different-Store canonical DAG activation reports
`external_store_import_not_implemented`, and the pre-fix Record probe reports
`same_store_import_not_implemented`.

## Attribution conclusion

The experiment measures visible prompt, Skill, Recall output, command count,
and wall time. It does not expose platform token telemetry for base
instructions, tool schemas, plugin discovery, model reasoning, cached input, or
billing. Text bytes are not token counts. No causal token percentage can
therefore be assigned to any of those hidden components.

There is enough evidence to avoid two speculative optimizations:

- do not double the default Recall budget merely to recover arbitrary completed
  history; pass a stable reference when that proof matters;
- do not trim governance or WorkVCS instructions on the assumption that they
  dominate token use.

Future prompt reduction should require platform telemetry or a controlled
behavioral regression showing that a specific instruction is redundant. The
current bridge pilot supports bounded Recall, explicit identity handoff, and
single-writer persistence as the lower-friction operating contract.

## Evidence boundary

This is one local, bounded multi-Agent case. It proves useful recovery and two
real currentness/portability defects; it does not establish general model
quality, exact token attribution, networked collaboration, or cross-Store
canonical history import.
