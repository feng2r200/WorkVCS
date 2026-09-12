use crate::canonical::CanonicalValue;
use crate::error::{Result, WorkVcsError};
use crate::history::{
    self, ApplicabilityResourceStampInput, EvidenceCreateOptions, EvidenceCreateResult,
    ResourceObservationCreateOptions, ResourceObservationCreateResult,
    VerificationApplicabilityCacheSnapshot, VerificationApplicabilityRecordOptions,
    VerificationCreateCommit, VerificationCreateOptions, VerificationResourceBasis,
    VerificationResult, VerificationTarget,
};
use crate::identity::{BranchId, CommitId};
use crate::store::StoreConnection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyResourceObservationInput {
    observation: ResourceObservationCreateOptions,
    scope_kind: String,
    scope_schema_version: i64,
    scope_payload: CanonicalValue,
}

impl VerifyResourceObservationInput {
    pub fn new(
        observation: ResourceObservationCreateOptions,
        scope_kind: impl Into<String>,
        scope_schema_version: i64,
        scope_payload: CanonicalValue,
    ) -> Result<Self> {
        let basis = VerificationResourceBasis::new(
            observation.resource_id(),
            observation.adapter_kind(),
            observation.adapter_schema_version(),
            scope_kind.into(),
            scope_schema_version,
            scope_payload,
            observation.fingerprint(),
        )?;
        Ok(Self {
            observation,
            scope_kind: basis.scope_kind,
            scope_schema_version: basis.scope_schema_version,
            scope_payload: basis.scope_payload,
        })
    }

    pub fn observation(&self) -> &ResourceObservationCreateOptions {
        &self.observation
    }

    fn resource_basis(
        &self,
        observation: &ResourceObservationCreateResult,
    ) -> Result<VerificationResourceBasis> {
        VerificationResourceBasis::new(
            self.observation.resource_id(),
            self.observation.adapter_kind(),
            self.observation.adapter_schema_version(),
            self.scope_kind.clone(),
            self.scope_schema_version,
            self.scope_payload.clone(),
            self.observation.fingerprint(),
        )?
        .with_baseline_observation_id(observation.observation_id)
    }

    fn applicability_stamp(
        &self,
        observation: &ResourceObservationCreateResult,
    ) -> Result<ApplicabilityResourceStampInput> {
        ApplicabilityResourceStampInput::observed(
            0,
            self.observation.adapter_kind(),
            self.observation.adapter_schema_version(),
            self.scope_schema_version,
            self.observation.fingerprint(),
        )?
        .with_observation_id(observation.observation_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyOptions {
    branch_id: BranchId,
    expected_head_commit_id: CommitId,
    target: VerificationTarget,
    result: VerificationResult,
    method: CanonicalValue,
    evidence: EvidenceCreateOptions,
    resource_observation: Option<VerifyResourceObservationInput>,
    cache_detail: CanonicalValue,
}

impl VerifyOptions {
    pub fn new(
        branch_id: BranchId,
        expected_head_commit_id: CommitId,
        target: VerificationTarget,
        result: VerificationResult,
        evidence: EvidenceCreateOptions,
    ) -> Result<Self> {
        Ok(Self {
            branch_id,
            expected_head_commit_id,
            target,
            result,
            method: CanonicalValue::object(Vec::new())?,
            evidence,
            resource_observation: None,
            cache_detail: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_method(mut self, method: CanonicalValue) -> Result<Self> {
        VerificationCreateOptions::new(
            self.branch_id,
            self.expected_head_commit_id,
            self.target,
            self.result,
        )?
        .with_method(method.clone())?;
        self.method = method;
        Ok(self)
    }

    pub fn with_resource_observation(
        mut self,
        resource_observation: VerifyResourceObservationInput,
    ) -> Self {
        self.resource_observation = Some(resource_observation);
        self
    }

    pub fn with_cache_detail(mut self, cache_detail: CanonicalValue) -> Result<Self> {
        if !matches!(cache_detail, CanonicalValue::Object(_)) {
            return Err(WorkVcsError::TaskInvalid(
                "verify cache detail must be a canonical object".to_owned(),
            ));
        }
        self.cache_detail = cache_detail;
        Ok(self)
    }

    pub(crate) fn evidence_mut(&mut self) -> &mut EvidenceCreateOptions {
        &mut self.evidence
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyResult {
    pub evidence: EvidenceCreateResult,
    pub resource_observation: Option<ResourceObservationCreateResult>,
    pub verification: VerificationCreateCommit,
    pub applicability_cache: Option<VerificationApplicabilityCacheSnapshot>,
}

pub(crate) fn verify(
    connection: &mut StoreConnection,
    options: &VerifyOptions,
) -> Result<VerifyResult> {
    preflight_verify(connection, options)?;

    let evidence = history::create_evidence(connection, &options.evidence)?;
    let resource_observation = options
        .resource_observation
        .as_ref()
        .map(|resource| history::record_resource_observation(connection, resource.observation()))
        .transpose()?;

    let mut verification_options = VerificationCreateOptions::new(
        options.branch_id,
        options.expected_head_commit_id,
        options.target,
        options.result,
    )?
    .with_method(options.method.clone())?
    .with_evidence(vec![evidence.evidence_id])?;

    if let Some((resource, observation)) = options
        .resource_observation
        .as_ref()
        .zip(resource_observation.as_ref())
    {
        verification_options = verification_options
            .with_resource_basis(vec![resource.resource_basis(observation)?])?;
    }

    let verification = history::create_verification(connection, &verification_options)?;

    let applicability_cache = match (
        options.resource_observation.as_ref(),
        resource_observation.as_ref(),
    ) {
        (Some(resource), Some(observation)) => {
            let stamp = resource.applicability_stamp(observation)?;
            Some(history::record_verification_applicability(
                connection,
                &VerificationApplicabilityRecordOptions::new(
                    options.branch_id,
                    verification.verification_entity_id,
                    verification.commit_id,
                )?
                .with_resource_stamps(vec![stamp])?
                .with_detail(options.cache_detail.clone())?,
            )?)
        }
        _ => None,
    };

    Ok(VerifyResult {
        evidence,
        resource_observation,
        verification,
        applicability_cache,
    })
}

pub(crate) fn preflight_verify(
    connection: &StoreConnection,
    options: &VerifyOptions,
) -> Result<()> {
    if options.resource_observation.is_none()
        && options.cache_detail != CanonicalValue::object(Vec::new())?
    {
        return Err(WorkVcsError::TaskInvalid(
            "verify cache detail requires a resource observation".to_owned(),
        ));
    }

    let branch = history::branch_head(connection, options.branch_id)?;
    if branch.head_commit_id != options.expected_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            options.branch_id, options.expected_head_commit_id, branch.head_commit_id
        )));
    }

    match options.target {
        VerificationTarget::AcceptanceCriterion(criterion_id) => {
            history::acceptance_criterion_at(
                connection,
                options.expected_head_commit_id,
                criterion_id,
            )?;
        }
        VerificationTarget::VerificationRequirement(requirement_id) => {
            history::verification_requirement_at(
                connection,
                options.expected_head_commit_id,
                requirement_id,
            )?;
        }
    }

    if let Some(resource) = &options.resource_observation {
        history::resource(connection, resource.observation.resource_id())?;
    }
    Ok(())
}
