use super::{
    GoalSnapshot, KnowledgeListOptions, KnowledgeRelationListOptions, KnowledgeRelationSnapshot,
    KnowledgeSnapshot, PlanSnapshot, RecordKnowledgeRelationListOptions,
    RecordKnowledgeRelationSnapshot, RecordListOptions, RecordRelationListOptions,
    RecordRelationSnapshot, RecordSnapshot, ReplayCache, TaskSnapshot, goal, knowledge, plan,
    record, state_at_with_cache, task,
};
use crate::error::Result;
use crate::identity::{CommitId, WorkspaceId};
use crate::store::StoreConnection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticSnapshotOptions {
    commit_id: CommitId,
    include_record_relations: bool,
    include_record_knowledge_relations: bool,
    include_knowledge_relations: bool,
}

impl SemanticSnapshotOptions {
    pub fn new(commit_id: CommitId) -> Self {
        Self {
            commit_id,
            include_record_relations: false,
            include_record_knowledge_relations: false,
            include_knowledge_relations: false,
        }
    }

    pub fn with_record_relations(mut self) -> Self {
        self.include_record_relations = true;
        self
    }

    pub fn with_record_knowledge_relations(mut self) -> Self {
        self.include_record_knowledge_relations = true;
        self
    }

    pub fn with_knowledge_relations(mut self) -> Self {
        self.include_knowledge_relations = true;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub goals: Vec<GoalSnapshot>,
    pub plans: Vec<PlanSnapshot>,
    pub tasks: Vec<TaskSnapshot>,
    pub records: Vec<RecordSnapshot>,
    pub knowledge: Vec<KnowledgeSnapshot>,
    pub record_relations: Vec<RecordRelationSnapshot>,
    pub record_knowledge_relations: Vec<RecordKnowledgeRelationSnapshot>,
    pub knowledge_relations: Vec<KnowledgeRelationSnapshot>,
}

pub(crate) fn semantic_snapshot(
    connection: &StoreConnection,
    options: &SemanticSnapshotOptions,
) -> Result<SemanticSnapshot> {
    let mut replay_cache = ReplayCache::default();
    let replayed = state_at_with_cache(connection, options.commit_id, &mut replay_cache)?;
    let goals = goal::goals_at_with_cache(connection, options.commit_id, &mut replay_cache)?;
    let plans = plan::plans_at_with_cache(connection, options.commit_id, &mut replay_cache)?;
    let tasks = task::tasks_at_with_cache(connection, options.commit_id, &mut replay_cache)?;
    let records = record::records_at_with_cache(
        connection,
        &RecordListOptions::new(options.commit_id),
        &mut replay_cache,
    )?
    .records;
    let knowledge = knowledge::knowledges_at_with_cache(
        connection,
        &KnowledgeListOptions::new(options.commit_id),
        &mut replay_cache,
    )?
    .knowledge;
    let record_relations = if options.include_record_relations {
        record::record_relations_at_with_cache(
            connection,
            &RecordRelationListOptions::new(options.commit_id),
            &mut replay_cache,
        )?
        .relations
    } else {
        Vec::new()
    };
    let record_knowledge_relations = if options.include_record_knowledge_relations {
        record::record_knowledge_relations_at_with_cache(
            connection,
            &RecordKnowledgeRelationListOptions::new(options.commit_id),
            &mut replay_cache,
        )?
        .relations
    } else {
        Vec::new()
    };
    let knowledge_relations = if options.include_knowledge_relations {
        record::knowledge_relations_at_with_cache(
            connection,
            &KnowledgeRelationListOptions::new(options.commit_id),
            &mut replay_cache,
        )?
        .relations
    } else {
        Vec::new()
    };

    Ok(SemanticSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id: options.commit_id,
        goals,
        plans,
        tasks,
        records,
        knowledge,
        record_relations,
        record_knowledge_relations,
        knowledge_relations,
    })
}
