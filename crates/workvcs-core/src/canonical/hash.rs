use crate::canonical::{CanonicalValue, WorkState, canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError};
use crate::identity::Digest;

const DOMAIN_ENTITY_VERSION: &str = "workvcs.entity-version.v1";
const DOMAIN_RELATION_VERSION: &str = "workvcs.relation-version.v1";
const DOMAIN_WORK_STATE: &str = "workvcs.work-state.v1";

pub fn entity_version_digest(value: &CanonicalValue) -> Result<Digest> {
    Ok(Digest::domain_separated(
        DOMAIN_ENTITY_VERSION,
        &canonical_bytes(value)?,
    ))
}

pub fn relation_version_digest(value: &CanonicalValue) -> Result<Digest> {
    Ok(Digest::domain_separated(
        DOMAIN_RELATION_VERSION,
        &canonical_bytes(value)?,
    ))
}

pub fn work_state_mapping_digest(state: &WorkState) -> Digest {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"workvcs-workstate-mapping-v1\0");

    let mut entities = state.entities().to_vec();
    entities.sort_by_key(|(entity_id, _)| entity_id.raw_bytes());
    payload.extend_from_slice(&(entities.len() as u64).to_be_bytes());
    for (entity_id, version_id) in entities {
        payload.extend_from_slice(&entity_id.raw_bytes());
        payload.extend_from_slice(&version_id.raw_bytes());
    }

    let mut relations = state.relations().to_vec();
    relations.sort_by_key(|(relation_id, _)| relation_id.raw_bytes());
    payload.extend_from_slice(&(relations.len() as u64).to_be_bytes());
    for (relation_id, version_id) in relations {
        payload.extend_from_slice(&relation_id.raw_bytes());
        payload.extend_from_slice(&version_id.raw_bytes());
    }

    Digest::domain_separated(DOMAIN_WORK_STATE, &payload)
}

pub fn content_object_digest(raw_bytes: &[u8]) -> Digest {
    Digest::raw(raw_bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportDigestDomain {
    EntityVersion,
    RelationVersion,
}

pub fn validate_import_fixed_point(
    canonical_json_bytes: &[u8],
    expected_digest: Digest,
    domain: ImportDigestDomain,
) -> Result<CanonicalValue> {
    let value = parse_canonical_json(canonical_json_bytes)?;
    let reencoded = canonical_bytes(&value)?;
    if reencoded != canonical_json_bytes {
        return Err(WorkVcsError::ImmutableImportInvalid(
            "stored bytes are not the fixed-point canonical representation".to_owned(),
        ));
    }
    let actual = match domain {
        ImportDigestDomain::EntityVersion => entity_version_digest(&value)?,
        ImportDigestDomain::RelationVersion => relation_version_digest(&value)?,
    };
    if actual != expected_digest {
        return Err(WorkVcsError::ImmutableImportInvalid(
            "expected digest does not match canonical bytes".to_owned(),
        ));
    }
    Ok(value)
}
