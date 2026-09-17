# Semantic recording

Choose the smallest truthful object:

| Meaning | WorkVCS object |
| --- | --- |
| desired outcome | Goal |
| durable route whose coordination or recovery value justifies persistence | Plan |
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

## Review currentness without inventing truth

Run `workvcs record currentness-audit --cwd <project>` when a semantic review
has concrete value—for example after resumed long-running work, at a meaningful
closeout, during a periodic retrospective, or when current Recall conflicts
with present evidence. The default returns only explicit open obligations:
unverified Assumptions, running Attempts, active Questions, and active Risks.
Use `--include-current-claims` to also review validated Assumptions and active
Decisions and Findings. Narrow large reviews with `--kind`, exact
`--scope-json`, or `--statement-contains`; keep the returned item budget
proportional to the review.

The command reports candidates, not detected errors. For each candidate,
compare its full statement and scope with current evidence and choose one of
the reported outcomes: retain it unchanged, or perform the appropriate guarded
status/correction operation with a truthful rationale. Do not close a Record
because its Plan or Task ended, and do not create changes merely to make an
audit return zero. Historical `STORE --commit` audits are inspection-only; use
the current Branch head and current Record version for any later mutation.
