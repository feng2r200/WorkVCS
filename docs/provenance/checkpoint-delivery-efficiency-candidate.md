# Checkpoint delivery efficiency source candidate

Status: validated local source delta; installation and live delivery excluded
Date: 2026-10-04

## Decision and boundary

The reported persistence gap does not establish a missing runtime capability.
The existing explicit recovery route can record an existing binding receipt,
deliver to that same target, and recover a committed-but-unreceipted result.
This candidate changes usage guidance and adds one focused behavioral test.
No runtime command, schema, resolver priority, activation gate, or delivery
contract changes. [ADR-0513](../decisions/adr/0513-projectref-durable-capture-routing.md)
and [ADR-0515](../decisions/adr/0515-capture-semantic-preflight-and-terminal-invalid-intent.md)
remain the governing contracts; automatic normal delivery is not introduced.

The Skill entrypoint routes checkpoint reconciliation to the conditional
[checkpoint delivery guide](../../skills/workvcs/references/checkpoint-delivery.md).
It distinguishes missing receipt from unbound ownership, and the status call's
own zero-write report from an existing delivery receipt. It retains current
scope and same-identity recovery, exact guards, truthful pending state, and
stable-ID readback. The reference is reused, not reread at every checkpoint.

The semantic reference also distinguishes a No-Plan test-result Finding from a
separate Verification entity. A rejected `records[].kind=verification` during
this round exposed that caller error before admission; the corrected record
retains its exact test evidence and does not require a manufactured Plan.

## Executed behavioral evidence

Twelve CLI tests passed using temporary registries, journals, and Stores:

| Test or group | Proven boundary |
| --- | --- |
| `cli_bound_checkpoint_recovers_lost_receipt_without_rebinding_or_duplicate` (new) | A resolved valid target can still report `pending_project` because its receipt is missing; a wrong projection guard writes no commit; a lost receipt after target commit is recovered under fresh public CLI guards; no rebinding or duplicate commit; completed status and brief Recall expose the semantic result |
| `capture_recovery` filter (5 existing tests) | Exact primary delivery, receipt reuse, fault convergence, valid handoff provenance, terminal invalid intent, and conflict without fallback |
| `cli_v2_unbound_semantic_locator_blocks_bound_cwd_fallback` | A stronger unbound semantic owner is not replaced with a convenient bound path |
| `cli_codex_project_adapter_routes_mirror_capture_to_semantic_owner_end_to_end` | A valid semantic target behind a mirror is retained for admission and recovery |
| `cli_v2_journal_admission_is_default_off_exactly_activated_and_store_free` | Admission alone stays Store-free behind its own exact gate |
| `capture_group` filter (3 existing tests) | Public group admission and recovery, immutable secondary references, missing-step recovery, and idempotent convergence |

Formatting, CLI test lint with warnings denied, source whitespace checks,
structural Skill validation, the 95-row ProjectRef contract matrix, and Skill
tree manifest creation and verification passed. The matrix is a contract check,
not 95 new runtime experiments. One initial exact test selector ran zero tests;
it was corrected to the qualified test name before counting the new test pass.

The new test lives under the existing test-only module. Fault injection and
routing activation occur only inside temporary fixtures. No installed binary,
real registry marker, existing user Store, or historical capture is changed.

## Applicability and next adoption boundary

These checks establish isolated existing-target command behavior and the
candidate's source integrity. They do not establish live adoption, delivery of
any audit-session record, or measured Token savings. Source approval does not
cover installing the Skill or delivering real captures; those need their
specific current scope.

For the later authorized round, integrate the reviewed source delta, identify
the exact Skill tree, check the installed state, and select only the approved
capture scope. Use fresh status guards and exact result readback. Do not sweep
history, bootstrap ownership, refresh markers, or activate another route as
an implicit consequence of this optimization.

Before adoption, rollback means discarding the isolated delta. It never means
resetting unrelated source work or deleting durable journal history.
