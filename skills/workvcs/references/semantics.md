# Semantic recording

Choose the smallest truthful object:

| Meaning | WorkVCS object |
| --- | --- |
| desired outcome | Goal |
| selected multi-step route | Plan |
| executable unit | Task |
| provisional belief | Assumption Record |
| observed fact or analysis result | Finding Record |
| selected choice and tradeoff | Decision Record |
| unresolved issue | Question Record |
| tried route, including failure | Attempt Record |
| immutable supporting material | Evidence |
| reusable conclusion beyond the immediate run | Knowledge |

Record enough scope and provenance to distinguish what was directly observed,
what was inferred, and what remains unknown. A useful retrospective should be
able to reconstruct:

- the problem and intended outcome;
- important assumptions, findings, decisions, and alternatives;
- failed attempts and why the route changed;
- evidence and verification results;
- unresolved questions and risks;
- Knowledge candidates and the records that support them.

Relations express meaning, not ordering decoration:

- `derived_from`: one record was reasoned from another;
- `supports` / `contradicts`: a Finding changes confidence in a Decision or
  Knowledge statement;
- `validates` / `invalidates`: evidence or a Finding confirms or rejects a
  claim under its recorded scope;
- `supersedes`: a newer Decision, Finding, or Knowledge statement replaces an
  older one;
- `related_to`: a labeled non-causal association when no stronger relation is
  justified.

`capture` can atomically create new Records, Knowledge, Evidence metadata, and
supported relations among newly named local items. Lifecycle-changing
`invalidates` and `supersedes` operations against existing objects use their
dedicated commands so the replaced target is explicit and guarded.

Finding currentness is not inferred from recency or text similarity:

- use `record supersede-finding` when an active correcting Finding replaces an
  active prior Finding;
- use `record invalidate-finding` when an active Finding disproves an active
  target Finding;
- both operations guard the Branch head and target Finding version, change the
  target state, and create the canonical relation atomically;
- terminal Findings remain available to retrospective and causal queries but
  are not current facts and cannot transition again.

Question and Risk currentness is also explicit:

- Question: `active -> answered | deferred | withdrawn`;
- Risk: `active -> mitigated | invalidated | withdrawn`;
- terminal Questions and Risks remain available to retrospective queries but
  are not current unknowns or exposures; renewed conditions create new
  Records.

`record attempt` starts a `running` Attempt. Once the route has a known result,
finish it with `record attempt-status` as `succeeded`, `failed`, or
`inconclusive`; do not leave a completed experiment mechanically running.
