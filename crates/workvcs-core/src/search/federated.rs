use super::semantic::{SemanticDerivedStatus, SemanticQueryResult, SemanticSearchItem};
use super::{
    SearchDerivedStatus, SearchItem, SearchQueryResult, SearchSourceIdentity, SearchSourceType,
    SearchTemporalScope, validate_query,
};
use crate::error::{Result, WorkVcsError};
use crate::identity::{CommitId, Digest, EntityId, ProjectRefId};
use serde::Serialize;
use std::collections::BTreeSet;

const FEDERATED_RANKING_POLICY_VERSION: &str = "source-reciprocal-rank-v1";
const RECIPROCAL_RANK_CONSTANT: f64 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FederatedRetrievalKind {
    Lexical,
    Semantic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FederatedSearchStatus {
    Ready,
    PartialFailure,
    Failed,
}

#[derive(Clone, Debug)]
pub struct FederatedQueryOptions {
    query: String,
    retrieval_kind: FederatedRetrievalKind,
    result_limit: Option<usize>,
}

impl FederatedQueryOptions {
    pub fn new(query: impl Into<String>, retrieval_kind: FederatedRetrievalKind) -> Result<Self> {
        let query = query.into();
        validate_query(&query)?;
        Ok(Self {
            query,
            retrieval_kind,
            result_limit: None,
        })
    }

    pub fn with_result_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "federated search result limit must be greater than zero".to_owned(),
            ));
        }
        self.result_limit = Some(limit);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FederatedSourceFailure {
    pub requested_project_ref_id: ProjectRefId,
    pub error_code: String,
    pub error: String,
}

impl FederatedSourceFailure {
    pub fn new(
        requested_project_ref_id: ProjectRefId,
        error_code: impl Into<String>,
        error: impl Into<String>,
    ) -> Result<Self> {
        let error_code = error_code.into();
        let error = error.into();
        if error_code.trim().is_empty() || error.trim().is_empty() {
            return Err(WorkVcsError::QueryInvalid(
                "federated source failure requires a non-empty code and message".to_owned(),
            ));
        }
        Ok(Self {
            requested_project_ref_id,
            error_code,
            error,
        })
    }
}

#[derive(Clone, Debug)]
pub enum FederatedSourceInput {
    Lexical(Box<SearchQueryResult>),
    Semantic(Box<SemanticQueryResult>),
    Failed(FederatedSourceFailure),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "retrieval_kind", content = "item", rename_all = "snake_case")]
pub enum FederatedItemPayload {
    Lexical(SearchItem),
    Semantic(SemanticSearchItem),
}

impl FederatedItemPayload {
    fn source_type(&self) -> SearchSourceType {
        match self {
            Self::Lexical(item) => item.source_type,
            Self::Semantic(item) => item.source_type,
        }
    }

    fn entity_id(&self) -> EntityId {
        match self {
            Self::Lexical(item) => item.entity_id,
            Self::Semantic(item) => item.entity_id,
        }
    }

    fn local_score(&self) -> f64 {
        match self {
            Self::Lexical(item) => item.score,
            Self::Semantic(item) => item.score,
        }
    }

    fn readback_verified(&self) -> bool {
        match self {
            Self::Lexical(item) => item.readback_verified,
            Self::Semantic(item) => item.readback_verified,
        }
    }

    fn serialized_tokens(&self) -> usize {
        match self {
            Self::Lexical(item) => item.serialized_tokens,
            Self::Semantic(item) => item.serialized_tokens,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FederatedSearchItem {
    pub source_identity: SearchSourceIdentity,
    pub temporal_scope: SearchTemporalScope,
    pub source_commit_id: CommitId,
    pub source_state_digest: Digest,
    pub local_rank: usize,
    pub local_score: f64,
    pub federated_score: f64,
    pub payload: FederatedItemPayload,
    pub readback_verified: bool,
    pub serialized_tokens: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FederatedSourceReport {
    Ready {
        source_identity: SearchSourceIdentity,
        temporal_scope: SearchTemporalScope,
        source_commit_id: CommitId,
        source_state_digest: Digest,
        candidates_considered: usize,
        matched_objects: usize,
        canonical_readback_verified: bool,
        derived_files_written: bool,
    },
    Failed {
        requested_project_ref_id: ProjectRefId,
        error_code: String,
        error: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FederatedQueryResult {
    pub status: FederatedSearchStatus,
    pub complete: bool,
    pub query: String,
    pub retrieval_kind: FederatedRetrievalKind,
    pub ranking_policy_version: String,
    pub reciprocal_rank_constant: f64,
    pub result_limit: Option<usize>,
    pub sources_requested: usize,
    pub sources_succeeded: usize,
    pub sources_failed: usize,
    pub candidates_considered: usize,
    pub matched_objects: usize,
    pub omitted_results: usize,
    pub serialized_item_tokens: usize,
    pub source_reports: Vec<FederatedSourceReport>,
    pub items: Vec<FederatedSearchItem>,
    pub canonical_readback_verified: bool,
    pub canonical_source_written: bool,
    pub derived_files_written: bool,
}

struct ReadySource {
    query: String,
    retrieval_kind: FederatedRetrievalKind,
    source_identity: SearchSourceIdentity,
    temporal_scope: SearchTemporalScope,
    source_commit_id: CommitId,
    source_state_digest: Digest,
    candidates_considered: usize,
    matched_objects: usize,
    canonical_readback_verified: bool,
    derived_files_written: bool,
    items: Vec<FederatedItemPayload>,
}

impl ReadySource {
    fn from_lexical(result: SearchQueryResult) -> Result<Self> {
        if result.status != SearchDerivedStatus::Ready
            || result.result_limit.is_some()
            || result.omitted_results != 0
            || !result.canonical_readback_verified
            || result.items.iter().any(|item| !item.readback_verified)
        {
            return Err(WorkVcsError::QueryInvalid(
                "federated lexical source must be a complete ready result with canonical readback"
                    .to_owned(),
            ));
        }
        if result.matched_objects != result.items.len() {
            return Err(WorkVcsError::QueryInvalid(
                "federated lexical source result count is incomplete".to_owned(),
            ));
        }
        Ok(Self {
            query: result.query,
            retrieval_kind: FederatedRetrievalKind::Lexical,
            source_identity: result.source_identity,
            temporal_scope: result.temporal_scope,
            source_commit_id: result.source_commit_id,
            source_state_digest: result.source_state_digest,
            candidates_considered: result.candidates_considered,
            matched_objects: result.matched_objects,
            canonical_readback_verified: result.canonical_readback_verified,
            derived_files_written: matches!(
                result.index_maintenance,
                Some(super::SearchRefreshOutcome::Refreshed)
            ),
            items: result
                .items
                .into_iter()
                .map(FederatedItemPayload::Lexical)
                .collect(),
        })
    }

    fn from_semantic(result: SemanticQueryResult) -> Result<Self> {
        if result.status != SemanticDerivedStatus::Ready
            || result.result_limit.is_some()
            || result.omitted_results != 0
            || !result.canonical_readback_verified
            || result.canonical_source_written
            || result.items.iter().any(|item| !item.readback_verified)
        {
            return Err(WorkVcsError::QueryInvalid(
                "federated semantic source must be a complete ready result with canonical readback"
                    .to_owned(),
            ));
        }
        if result.matched_objects != result.items.len() {
            return Err(WorkVcsError::QueryInvalid(
                "federated semantic source result count is incomplete".to_owned(),
            ));
        }
        Ok(Self {
            query: result.query,
            retrieval_kind: FederatedRetrievalKind::Semantic,
            source_identity: result.source_identity,
            temporal_scope: result.temporal_scope,
            source_commit_id: result.source_commit_id,
            source_state_digest: result.source_state_digest,
            candidates_considered: result.candidates_considered,
            matched_objects: result.matched_objects,
            canonical_readback_verified: result.canonical_readback_verified,
            derived_files_written: result.derived_files_written,
            items: result
                .items
                .into_iter()
                .map(FederatedItemPayload::Semantic)
                .collect(),
        })
    }
}

pub fn aggregate_federated_search(
    options: FederatedQueryOptions,
    inputs: Vec<FederatedSourceInput>,
) -> Result<FederatedQueryResult> {
    if inputs.len() < 2 {
        return Err(WorkVcsError::QueryInvalid(
            "federated search requires at least two explicit sources".to_owned(),
        ));
    }
    let mut ready_sources = Vec::new();
    let mut source_reports = Vec::with_capacity(inputs.len());
    let mut project_refs = BTreeSet::new();
    for input in inputs {
        match input {
            FederatedSourceInput::Lexical(result) => {
                let ready = ReadySource::from_lexical(*result)?;
                require_ready_source(&options, &ready, &mut project_refs)?;
                source_reports.push(ready_report(&ready));
                ready_sources.push(ready);
            }
            FederatedSourceInput::Semantic(result) => {
                let ready = ReadySource::from_semantic(*result)?;
                require_ready_source(&options, &ready, &mut project_refs)?;
                source_reports.push(ready_report(&ready));
                ready_sources.push(ready);
            }
            FederatedSourceInput::Failed(failure) => {
                require_unique_project_ref(failure.requested_project_ref_id, &mut project_refs)?;
                source_reports.push(FederatedSourceReport::Failed {
                    requested_project_ref_id: failure.requested_project_ref_id,
                    error_code: failure.error_code,
                    error: failure.error,
                });
            }
        }
    }

    let sources_requested = project_refs.len();
    let sources_succeeded = ready_sources.len();
    let sources_failed = sources_requested.saturating_sub(sources_succeeded);
    let candidates_considered = ready_sources
        .iter()
        .map(|source| source.candidates_considered)
        .sum();
    let derived_files_written = ready_sources
        .iter()
        .any(|source| source.derived_files_written);
    let mut seen_objects = BTreeSet::new();
    let mut items = Vec::new();
    for source in ready_sources {
        for (offset, payload) in source.items.into_iter().enumerate() {
            let local_rank = offset + 1;
            let key = format!(
                "{}\0{}\0{}\0{}\0{}\0{}",
                source.source_identity.project_ref_id,
                source.source_identity.store_id,
                source.source_identity.workspace_id,
                source.source_identity.branch_id,
                payload.source_type().as_str(),
                payload.entity_id(),
            );
            if !seen_objects.insert(key) {
                return Err(WorkVcsError::QueryInvalid(
                    "federated source contains a duplicate object identity".to_owned(),
                ));
            }
            let local_score = payload.local_score();
            let readback_verified = payload.readback_verified();
            let serialized_tokens = payload.serialized_tokens();
            items.push(FederatedSearchItem {
                source_identity: source.source_identity,
                temporal_scope: source.temporal_scope,
                source_commit_id: source.source_commit_id,
                source_state_digest: source.source_state_digest,
                local_rank,
                local_score,
                federated_score: 1.0 / (RECIPROCAL_RANK_CONSTANT + local_rank as f64),
                payload,
                readback_verified,
                serialized_tokens,
            });
        }
    }
    items.sort_by(|left, right| {
        right
            .federated_score
            .total_cmp(&left.federated_score)
            .then_with(|| {
                left.source_identity
                    .project_ref_id
                    .cmp(&right.source_identity.project_ref_id)
            })
            .then_with(|| {
                left.source_identity
                    .store_id
                    .cmp(&right.source_identity.store_id)
            })
            .then_with(|| left.payload.entity_id().cmp(&right.payload.entity_id()))
    });
    let matched_objects = items.len();
    let omitted_results = options
        .result_limit
        .map(|limit| matched_objects.saturating_sub(limit))
        .unwrap_or_default();
    if let Some(limit) = options.result_limit {
        items.truncate(limit);
    }
    let serialized_item_tokens = items.iter().map(|item| item.serialized_tokens).sum();
    let status = match (sources_succeeded, sources_failed) {
        (0, _) => FederatedSearchStatus::Failed,
        (_, 0) => FederatedSearchStatus::Ready,
        _ => FederatedSearchStatus::PartialFailure,
    };
    Ok(FederatedQueryResult {
        status,
        complete: sources_failed == 0,
        query: options.query,
        retrieval_kind: options.retrieval_kind,
        ranking_policy_version: FEDERATED_RANKING_POLICY_VERSION.to_owned(),
        reciprocal_rank_constant: RECIPROCAL_RANK_CONSTANT,
        result_limit: options.result_limit,
        sources_requested,
        sources_succeeded,
        sources_failed,
        candidates_considered,
        matched_objects,
        omitted_results,
        serialized_item_tokens,
        source_reports,
        items,
        canonical_readback_verified: sources_succeeded > 0,
        canonical_source_written: false,
        derived_files_written,
    })
}

fn require_ready_source(
    options: &FederatedQueryOptions,
    source: &ReadySource,
    project_refs: &mut BTreeSet<ProjectRefId>,
) -> Result<()> {
    if source.query != options.query || source.retrieval_kind != options.retrieval_kind {
        return Err(WorkVcsError::QueryInvalid(
            "federated sources must use the exact same query and retrieval kind".to_owned(),
        ));
    }
    if !source.canonical_readback_verified || source.matched_objects != source.items.len() {
        return Err(WorkVcsError::QueryInvalid(
            "federated source is not a complete canonical readback".to_owned(),
        ));
    }
    require_unique_project_ref(source.source_identity.project_ref_id, project_refs)
}

fn require_unique_project_ref(
    project_ref_id: ProjectRefId,
    project_refs: &mut BTreeSet<ProjectRefId>,
) -> Result<()> {
    if project_refs.insert(project_ref_id) {
        Ok(())
    } else {
        Err(WorkVcsError::QueryInvalid(format!(
            "federated source ProjectRef {project_ref_id} was selected more than once"
        )))
    }
}

fn ready_report(source: &ReadySource) -> FederatedSourceReport {
    FederatedSourceReport::Ready {
        source_identity: source.source_identity,
        temporal_scope: source.temporal_scope,
        source_commit_id: source.source_commit_id,
        source_state_digest: source.source_state_digest,
        candidates_considered: source.candidates_considered,
        matched_objects: source.matched_objects,
        canonical_readback_verified: source.canonical_readback_verified,
        derived_files_written: source.derived_files_written,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{BranchId, EntityVersionId, StoreId, WorkspaceId};

    fn identity(
        project_ref_id: ProjectRefId,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
    ) -> SearchSourceIdentity {
        SearchSourceIdentity::new(project_ref_id, store_id, workspace_id, branch_id)
    }

    fn lexical_item(entity_id: EntityId, score: f64) -> FederatedItemPayload {
        FederatedItemPayload::Lexical(SearchItem {
            source_id: format!("record:{entity_id}"),
            source_type: SearchSourceType::Record,
            entity_id,
            entity_version_id: EntityVersionId::new_v7(),
            entity_state_digest: Digest::from_bytes([1; 32]),
            statement_digest: Digest::from_bytes([2; 32]),
            status: "accepted".to_owned(),
            record_kind: Some("finding".to_owned()),
            scope_json: "{}".to_owned(),
            provenance_json: None,
            source_commit_id: CommitId::new_v7(),
            temporal_scope: SearchTemporalScope::Current,
            canonical_statement: "shared evidence".to_owned(),
            ranges: Vec::new(),
            score,
            readback_verified: true,
            serialized_tokens: 4,
        })
    }

    fn ready(
        identity: SearchSourceIdentity,
        items: Vec<FederatedItemPayload>,
    ) -> FederatedSourceInput {
        FederatedSourceInput::Lexical(Box::new(SearchQueryResult {
            status: SearchDerivedStatus::Ready,
            reason_code: None,
            reason: None,
            guidance: None,
            index_path: std::path::PathBuf::from("/tmp/index"),
            manifest: None,
            source_identity: identity,
            temporal_scope: SearchTemporalScope::Current,
            source_commit_id: CommitId::new_v7(),
            source_state_digest: Digest::from_bytes([3; 32]),
            query: "shared evidence".to_owned(),
            result_limit: None,
            token_count_encoding: "o200k_base".to_owned(),
            serialized_item_tokens: items.iter().map(|item| item.serialized_tokens()).sum(),
            candidates_considered: items.len(),
            matched_objects: items.len(),
            omitted_results: 0,
            items: items
                .into_iter()
                .map(|item| match item {
                    FederatedItemPayload::Lexical(item) => item,
                    FederatedItemPayload::Semantic(_) => unreachable!(),
                })
                .collect(),
            canonical_readback_verified: true,
            index_maintenance: None,
        }))
    }

    #[test]
    fn preserves_shared_and_isolated_store_identity_and_duplicate_entity_ids() {
        let shared_store = StoreId::new_v7();
        let shared_branch = BranchId::new_v7();
        let duplicate_entity = EntityId::new_v7();
        let first = identity(
            ProjectRefId::new_v7(),
            shared_store,
            WorkspaceId::new_v7(),
            shared_branch,
        );
        let second = identity(
            ProjectRefId::new_v7(),
            shared_store,
            WorkspaceId::new_v7(),
            shared_branch,
        );
        let third = identity(
            ProjectRefId::new_v7(),
            StoreId::new_v7(),
            WorkspaceId::new_v7(),
            shared_branch,
        );
        let result = aggregate_federated_search(
            FederatedQueryOptions::new("shared evidence", FederatedRetrievalKind::Lexical)
                .expect("options"),
            vec![
                ready(first, vec![lexical_item(duplicate_entity, 9.0)]),
                ready(second, vec![lexical_item(duplicate_entity, 8.0)]),
                ready(third, vec![lexical_item(duplicate_entity, 7.0)]),
            ],
        )
        .expect("federated result");
        assert_eq!(result.status, FederatedSearchStatus::Ready);
        assert!(result.complete);
        assert_eq!(result.sources_succeeded, 3);
        assert_eq!(result.matched_objects, 3);
        assert_eq!(
            result
                .items
                .iter()
                .map(|item| {
                    format!(
                        "{}:{}:{}:{}",
                        item.source_identity.project_ref_id,
                        item.source_identity.store_id,
                        item.source_identity.workspace_id,
                        item.source_identity.branch_id
                    )
                })
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
        assert!(!result.canonical_source_written);
    }

    #[test]
    fn reports_partial_failure_and_rejects_duplicate_source_or_object() {
        let first = identity(
            ProjectRefId::new_v7(),
            StoreId::new_v7(),
            WorkspaceId::new_v7(),
            BranchId::new_v7(),
        );
        let failed_project = ProjectRefId::new_v7();
        let failure = FederatedSourceFailure::new(
            failed_project,
            "source_not_authorized_or_unknown",
            "source was not present in the active verified registry",
        )
        .expect("failure");
        let result = aggregate_federated_search(
            FederatedQueryOptions::new("shared evidence", FederatedRetrievalKind::Lexical)
                .expect("options"),
            vec![
                ready(first, vec![lexical_item(EntityId::new_v7(), 1.0)]),
                FederatedSourceInput::Failed(failure),
            ],
        )
        .expect("partial result");
        assert_eq!(result.status, FederatedSearchStatus::PartialFailure);
        assert!(!result.complete);
        assert_eq!(result.sources_failed, 1);
        assert_eq!(result.source_reports.len(), 2);

        let duplicate_source = aggregate_federated_search(
            FederatedQueryOptions::new("shared evidence", FederatedRetrievalKind::Lexical)
                .expect("options"),
            vec![
                ready(first, vec![lexical_item(EntityId::new_v7(), 1.0)]),
                ready(first, vec![lexical_item(EntityId::new_v7(), 1.0)]),
            ],
        );
        assert!(duplicate_source.is_err());

        let entity = EntityId::new_v7();
        let duplicate_object = aggregate_federated_search(
            FederatedQueryOptions::new("shared evidence", FederatedRetrievalKind::Lexical)
                .expect("options"),
            vec![
                ready(
                    first,
                    vec![lexical_item(entity, 2.0), lexical_item(entity, 1.0)],
                ),
                FederatedSourceInput::Failed(
                    FederatedSourceFailure::new(
                        ProjectRefId::new_v7(),
                        "stale_binding",
                        "Store identity mismatch",
                    )
                    .expect("failure"),
                ),
            ],
        );
        assert!(duplicate_object.is_err());
    }

    #[test]
    fn applies_only_a_final_output_limit_after_complete_source_results() {
        let first = identity(
            ProjectRefId::new_v7(),
            StoreId::new_v7(),
            WorkspaceId::new_v7(),
            BranchId::new_v7(),
        );
        let second = identity(
            ProjectRefId::new_v7(),
            StoreId::new_v7(),
            WorkspaceId::new_v7(),
            BranchId::new_v7(),
        );
        let result = aggregate_federated_search(
            FederatedQueryOptions::new("shared evidence", FederatedRetrievalKind::Lexical)
                .expect("options")
                .with_result_limit(2)
                .expect("limit"),
            vec![
                ready(
                    first,
                    vec![
                        lexical_item(EntityId::new_v7(), 5.0),
                        lexical_item(EntityId::new_v7(), 4.0),
                    ],
                ),
                ready(
                    second,
                    vec![
                        lexical_item(EntityId::new_v7(), 3.0),
                        lexical_item(EntityId::new_v7(), 2.0),
                    ],
                ),
            ],
        )
        .expect("federated result");
        assert_eq!(result.candidates_considered, 4);
        assert_eq!(result.matched_objects, 4);
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.omitted_results, 2);
        assert!(result.items.iter().all(|item| item.local_rank == 1));
    }
}
