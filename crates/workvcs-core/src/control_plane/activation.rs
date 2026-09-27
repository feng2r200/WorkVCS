use super::model::{ControlPlaneDigest, ProjectRegistryV2, canonicalize_serializable, invalid};
use crate::RegistryId;
use crate::canonical::parse_canonical_json;
use crate::error::{Result, WorkVcsError};
use serde::{Deserialize, Serialize};

const ROUTING_ACTIVATION_VERSION: u64 = 1;
const JOURNAL_ADMISSION_ACTIVATION_VERSION: u64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingActivationScope {
    ProjectRefV2ReadRouting,
}

impl RoutingActivationScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectRefV2ReadRouting => "project_ref_v2_read_routing",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingActivationCandidate {
    activation_version: u64,
    scope: RoutingActivationScope,
    registry_id: RegistryId,
    registry_revision: u64,
    registry_digest: ControlPlaneDigest,
}

impl RoutingActivationCandidate {
    pub fn for_registry(registry: &ProjectRegistryV2) -> Result<Self> {
        registry.validate()?;
        Ok(Self {
            activation_version: ROUTING_ACTIVATION_VERSION,
            scope: RoutingActivationScope::ProjectRefV2ReadRouting,
            registry_id: registry.registry_id(),
            registry_revision: registry.revision(),
            registry_digest: registry.digest()?,
        })
    }

    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        parse_canonical_json(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "routing activation candidate is not strict canonical-domain JSON: {error}"
            ))
        })?;
        let candidate: Self = serde_json::from_slice(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "routing activation candidate has an invalid shape: {error}"
            ))
        })?;
        candidate.validate()?;
        Ok(candidate)
    }

    pub fn validate(&self) -> Result<()> {
        if self.activation_version != ROUTING_ACTIVATION_VERSION {
            return invalid(format!(
                "routing activation version {} is not supported; expected {ROUTING_ACTIVATION_VERSION}",
                self.activation_version
            ));
        }
        Ok(())
    }

    pub fn validate_registry(&self, registry: &ProjectRegistryV2) -> Result<()> {
        registry.validate()?;
        let actual_digest = registry.digest()?;
        if self.registry_id != registry.registry_id()
            || self.registry_revision != registry.revision()
            || self.registry_digest != actual_digest
        {
            return invalid(format!(
                "routing activation candidate is stale or belongs to another registry snapshot: candidate registry_id={} revision={} digest={}, actual registry_id={} revision={} digest={}",
                self.registry_id,
                self.registry_revision,
                self.registry_digest,
                registry.registry_id(),
                registry.revision(),
                actual_digest
            ));
        }
        Ok(())
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        canonicalize_serializable(self, "routing activation candidate")
    }

    pub fn stored_json_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = self.canonical_json_bytes()?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&self.canonical_json_bytes()?))
    }

    pub fn activation_version(&self) -> u64 {
        self.activation_version
    }

    pub fn scope(&self) -> RoutingActivationScope {
        self.scope
    }

    pub fn registry_id(&self) -> RegistryId {
        self.registry_id
    }

    pub fn registry_revision(&self) -> u64 {
        self.registry_revision
    }

    pub fn registry_digest(&self) -> &ControlPlaneDigest {
        &self.registry_digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalAdmissionActivationScope {
    ProjectRefV2JournalAdmission,
}

impl JournalAdmissionActivationScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectRefV2JournalAdmission => "project_ref_v2_journal_admission",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalAdmissionActivationCandidate {
    activation_version: u64,
    scope: JournalAdmissionActivationScope,
    registry_id: RegistryId,
    registry_revision: u64,
    registry_digest: ControlPlaneDigest,
}

impl JournalAdmissionActivationCandidate {
    pub fn for_registry(registry: &ProjectRegistryV2) -> Result<Self> {
        registry.validate()?;
        Ok(Self {
            activation_version: JOURNAL_ADMISSION_ACTIVATION_VERSION,
            scope: JournalAdmissionActivationScope::ProjectRefV2JournalAdmission,
            registry_id: registry.registry_id(),
            registry_revision: registry.revision(),
            registry_digest: registry.digest()?,
        })
    }

    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        parse_canonical_json(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "journal-admission activation candidate is not strict canonical-domain JSON: {error}"
            ))
        })?;
        let candidate: Self = serde_json::from_slice(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "journal-admission activation candidate has an invalid shape: {error}"
            ))
        })?;
        candidate.validate()?;
        Ok(candidate)
    }

    pub fn validate(&self) -> Result<()> {
        if self.activation_version != JOURNAL_ADMISSION_ACTIVATION_VERSION {
            return invalid(format!(
                "journal-admission activation version {} is not supported; expected {JOURNAL_ADMISSION_ACTIVATION_VERSION}",
                self.activation_version
            ));
        }
        Ok(())
    }

    pub fn validate_registry(&self, registry: &ProjectRegistryV2) -> Result<()> {
        registry.validate()?;
        let actual_digest = registry.digest()?;
        if self.registry_id != registry.registry_id()
            || self.registry_revision != registry.revision()
            || self.registry_digest != actual_digest
        {
            return invalid(format!(
                "journal-admission activation candidate is stale or belongs to another registry snapshot: candidate registry_id={} revision={} digest={}, actual registry_id={} revision={} digest={}",
                self.registry_id,
                self.registry_revision,
                self.registry_digest,
                registry.registry_id(),
                registry.revision(),
                actual_digest
            ));
        }
        Ok(())
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        canonicalize_serializable(self, "journal-admission activation candidate")
    }

    pub fn stored_json_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = self.canonical_json_bytes()?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&self.canonical_json_bytes()?))
    }

    pub fn activation_version(&self) -> u64 {
        self.activation_version
    }

    pub fn scope(&self) -> JournalAdmissionActivationScope {
        self.scope
    }

    pub fn registry_id(&self) -> RegistryId {
        self.registry_id
    }

    pub fn registry_revision(&self) -> u64 {
        self.registry_revision
    }

    pub fn registry_digest(&self) -> &ControlPlaneDigest {
        &self.registry_digest
    }
}
