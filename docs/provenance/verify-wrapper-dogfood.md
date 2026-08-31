# Verify Wrapper Dogfood Evidence

Status: Phase 4KY local dogfood evidence
Recorded: 2026-08-31

This file records a non-temporary local dogfood probe for the deterministic
single-target `verify` wrapper. The Store file is local runtime data and is
ignored by Git:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/runtime/dogfood/20260831-phase4ky-verify-wrapper.sqlite
```

The probe used the local Phase 4KY CLI build to:

1. initialize a WorkVCS Store;
2. create one Workspace and one Task representing the Phase 4KY wrapper slice;
3. create one required Acceptance Criterion;
4. create one filesystem Resource;
5. run one `workvcs verify` command that created Evidence, recorded one Resource
   Observation, created the Verification, and recorded the applicability cache;
6. confirm the Acceptance Criterion was `verified` at the Verification commit;
7. transition the Task to `done`;
8. observe the post-completion branch-head cache staleness boundary; and
9. refresh the applicability cache at the new branch head and confirm the
   Acceptance Criterion returned to `verified`.

Observed wrapper result:

```text
dogfood_result=passed
dogfood_workspace_id=01a05858-32be-7bf1-a2a6-3f5e5d76d4c1
dogfood_branch_id=01a05858-32be-7bf1-a2a6-3f8f16429643
dogfood_task_id=01a05858-3547-7ef2-a67b-1f719b3c71da
dogfood_criterion_id=01a05858-37da-75a0-a39e-1001fd043378
dogfood_evidence_id=01a05858-3d43-78a1-b821-a2744642ea77
dogfood_observation_id=01a05858-3d44-78f2-98bd-4034ded9fe95
dogfood_resource_fingerprint=2c73f3f51a7fc586e7541ecc8b11b4908f8bd3a70399142f8d621b29544fff53
dogfood_verification_id=01a05858-3d46-7181-9d9c-6a4c70ac04cf
dogfood_verified_commit_id=01a05858-3d46-7181-9d9c-6a9c5680bacd
dogfood_status_before_completion=verified
dogfood_cache_applicability=applicable
dogfood_cache_reason_code=all_basis_applicable
```

Observed post-completion recovery:

```text
dogfood_final_commit_id=01a05858-4502-7490-9243-8412257bcaa6
dogfood_status_after_completion=stale
dogfood_cache_refresh_evaluated_commit=01a05858-4502-7490-9243-8412257bcaa6
dogfood_status_after_refresh=verified
dogfood_doctor_valid_required=true
```

Parser isolation recheck after the Phase 4KY CLI stack fix:

```text
dogfood_result=passed
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/runtime/dogfood/20260831-phase4ky-verify-wrapper-recheck.sqlite
dogfood_workspace_id=01a0586d-fb2b-7df2-9c49-757b007b8fdf
dogfood_branch_id=01a0586d-fb2b-7df2-9c49-75a58dd66e1b
dogfood_task_id=01a0586d-fdc2-74d1-af79-6c74362a2a1d
dogfood_criterion_id=01a0586e-0058-7702-8770-93aa9e963178
dogfood_evidence_id=01a0586e-0808-76e2-b0ab-efc94d19a107
dogfood_observation_id=01a0586e-0809-7903-b557-b64c1e79d2dc
dogfood_resource_fingerprint=fbe7ab5afca3a9784e7426e4a0f09046e54d28743a5496a85f8ccb3a9cc23b8f
dogfood_verification_id=01a0586e-080b-7e80-ad50-72be1e2518af
dogfood_verified_commit_id=01a0586e-080b-7e80-ad50-7300ec5c1591
dogfood_final_commit_id=01a0586e-0fd7-7761-be21-171230c7667a
dogfood_status_before_completion=verified
dogfood_status_after_completion=stale
dogfood_status_after_refresh=verified
dogfood_cache_applicability=applicable
dogfood_cache_reason_code=all_basis_applicable
dogfood_doctor_valid_required=true
```

Residual gap at Phase 4KY:

This is dogfood proof for the bounded wrapper command and its direct
Evidence/Resource/Verification/cache composition. It is not proof that V1
Resource observation production, path/glob normalization, or adapter-backed
cache refresh is complete. The probe exposed a concrete recovery gap: after a
later WorkState commit advances the branch head, a Resource-backed Verification
becomes stale until the applicability cache is explicitly refreshed for the new
head. ADR-0415 closes the baseline-observation refresh path for this gap while
leaving external Resource re-observation outside V1 until Resource adapter/path
normalization is confirmed.
