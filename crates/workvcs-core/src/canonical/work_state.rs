use crate::error::{Result, WorkVcsError};
use crate::identity::{EntityId, EntityVersionId, RelationId, RelationVersionId};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkState {
    entities: Vec<(EntityId, EntityVersionId)>,
    relations: Vec<(RelationId, RelationVersionId)>,
}

impl WorkState {
    pub fn new(
        entities: impl IntoIterator<Item = (EntityId, EntityVersionId)>,
        relations: impl IntoIterator<Item = (RelationId, RelationVersionId)>,
    ) -> Result<Self> {
        let entities = entities.into_iter().collect::<Vec<_>>();
        let relations = relations.into_iter().collect::<Vec<_>>();
        reject_duplicate_entities(&entities)?;
        reject_duplicate_relations(&relations)?;
        Ok(Self {
            entities,
            relations,
        })
    }

    pub fn empty() -> Self {
        Self {
            entities: Vec::new(),
            relations: Vec::new(),
        }
    }

    pub fn entities(&self) -> &[(EntityId, EntityVersionId)] {
        &self.entities
    }

    pub fn relations(&self) -> &[(RelationId, RelationVersionId)] {
        &self.relations
    }
}

fn reject_duplicate_entities(entries: &[(EntityId, EntityVersionId)]) -> Result<()> {
    let mut seen = HashSet::new();
    for (entity_id, _) in entries {
        if !seen.insert(entity_id.raw_bytes()) {
            return Err(WorkVcsError::CanonicalEncodingInvalid(
                "WorkState entity mapping contains a duplicate EntityId".to_owned(),
            ));
        }
    }
    Ok(())
}

fn reject_duplicate_relations(entries: &[(RelationId, RelationVersionId)]) -> Result<()> {
    let mut seen = HashSet::new();
    for (relation_id, _) in entries {
        if !seen.insert(relation_id.raw_bytes()) {
            return Err(WorkVcsError::CanonicalEncodingInvalid(
                "WorkState relation mapping contains a duplicate RelationId".to_owned(),
            ));
        }
    }
    Ok(())
}
