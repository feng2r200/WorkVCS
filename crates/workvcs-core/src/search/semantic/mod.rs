mod generation;
mod model;
mod profile;
mod window;

use super::{SearchSourceIdentity, SearchSourceType, SearchTemporalScope};
use crate::error::Result;
use crate::identity::{CommitId, Digest, EntityId, EntityVersionId};
use crate::store::StoreConnection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticModelProfile {
    MultilingualE5LargeFp32V1,
    BgeRerankerV2M3Fp32V1,
}

impl SemanticModelProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MultilingualE5LargeFp32V1 => "multilingual-e5-large-fp32-v1",
            Self::BgeRerankerV2M3Fp32V1 => "bge-reranker-v2-m3-fp32-v1",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "multilingual-e5-large-fp32-v1" => Ok(Self::MultilingualE5LargeFp32V1),
            "bge-reranker-v2-m3-fp32-v1" => Ok(Self::BgeRerankerV2M3Fp32V1),
            _ => Err(crate::error::WorkVcsError::QueryInvalid(format!(
                "unsupported semantic model profile {value:?}"
            ))),
        }
    }
}

impl std::fmt::Display for SemanticModelProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticModelKind {
    Embedding,
    Reranker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticRankingMode {
    Vector,
    Rerank,
    HybridRrf,
    RerankVectorRrf,
    RerankHybridRrf,
}

impl SemanticRankingMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::Rerank => "rerank",
            Self::HybridRrf => "hybrid-rrf",
            Self::RerankVectorRrf => "rerank-vector-rrf",
            Self::RerankHybridRrf => "rerank-hybrid-rrf",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "vector" => Ok(Self::Vector),
            "rerank" => Ok(Self::Rerank),
            "hybrid-rrf" => Ok(Self::HybridRrf),
            "rerank-vector-rrf" => Ok(Self::RerankVectorRrf),
            "rerank-hybrid-rrf" => Ok(Self::RerankHybridRrf),
            _ => Err(crate::error::WorkVcsError::QueryInvalid(format!(
                "unsupported semantic ranking mode {value:?}"
            ))),
        }
    }

    pub const fn needs_reranker(self) -> bool {
        matches!(
            self,
            Self::Rerank | Self::RerankVectorRrf | Self::RerankHybridRrf
        )
    }

    pub const fn needs_lexical(self) -> bool {
        matches!(self, Self::HybridRrf | Self::RerankHybridRrf)
    }
}

impl std::fmt::Display for SemanticRankingMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticModelFileManifest {
    pub name: String,
    pub sha256: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticModelManifest {
    pub format_version: u32,
    pub profile: SemanticModelProfile,
    pub kind: SemanticModelKind,
    pub source: String,
    pub revision: String,
    pub files: Vec<SemanticModelFileManifest>,
    pub max_tokens: usize,
    pub dimension: Option<usize>,
    pub query_prefix: Option<String>,
    pub passage_prefix: Option<String>,
    pub pooling: Option<String>,
    pub output_key: String,
    pub fastembed_version: String,
    pub ort_version: String,
    pub created_at_us: i64,
    pub manifest_digest: Digest,
}

#[derive(Clone, Debug)]
pub struct SemanticModelProvisionOptions {
    profile: SemanticModelProfile,
    source_directory: PathBuf,
    destination: PathBuf,
}

impl SemanticModelProvisionOptions {
    pub fn new(
        profile: SemanticModelProfile,
        source_directory: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
    ) -> Self {
        Self {
            profile,
            source_directory: source_directory.into(),
            destination: destination.into(),
        }
    }

    pub fn profile(&self) -> SemanticModelProfile {
        self.profile
    }

    pub fn source_directory(&self) -> &Path {
        &self.source_directory
    }

    pub fn destination(&self) -> &Path {
        &self.destination
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticModelProvisionOutcome {
    Provisioned,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SemanticModelProvisionResult {
    pub outcome: SemanticModelProvisionOutcome,
    pub model_path: PathBuf,
    pub manifest: SemanticModelManifest,
    pub inference_canary_passed: bool,
    pub network_access_used: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticPassageCatalogEntry {
    pub point_id: u64,
    pub source_type: SearchSourceType,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub entity_state_digest: Digest,
    pub statement_digest: Digest,
    pub status: String,
    pub record_kind: Option<String>,
    pub scope_json: String,
    pub provenance_json: Option<String>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub tokenizer_token_start: usize,
    pub tokenizer_token_end: usize,
    pub model_tokens: usize,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticGenerationManifest {
    pub format_version: u32,
    pub ranking_policy_version: String,
    pub window_policy_version: String,
    pub identity: SearchSourceIdentity,
    pub temporal_scope: SearchTemporalScope,
    pub source_commit_id: CommitId,
    pub source_state_digest: Digest,
    pub object_version_basis_digest: Digest,
    pub object_count: usize,
    pub point_count: usize,
    pub embedding_model_manifest_digest: Digest,
    pub embedding_profile: SemanticModelProfile,
    pub dimension: usize,
    pub distance: String,
    pub vector_name: String,
    pub exact_search: bool,
    pub request_limit_rule: String,
    pub quantization: String,
    pub qdrant_edge_version: String,
    pub fastembed_version: String,
    pub ort_version: String,
    pub rust_target: String,
    pub provider: String,
    pub qdrant_config: serde_json::Value,
    pub qdrant_search_threads: usize,
    pub optimize_returned: bool,
    pub catalog_digest: Digest,
    pub point_id_digest: Digest,
    pub created_at_us: i64,
    pub basis_digest: Digest,
}

#[derive(Clone, Debug)]
pub struct SemanticQueryOptions {
    identity: SearchSourceIdentity,
    generation_root: PathBuf,
    embedding_profile: SemanticModelProfile,
    lexical_root: Option<PathBuf>,
    embedding_model_path: PathBuf,
    reranker_profile: Option<SemanticModelProfile>,
    reranker_model_path: Option<PathBuf>,
    query: String,
    ranking_mode: SemanticRankingMode,
    commit_id: Option<CommitId>,
    result_limit: Option<usize>,
    protected_paths: Vec<PathBuf>,
}

impl SemanticQueryOptions {
    pub fn new(
        identity: SearchSourceIdentity,
        generation_root: impl Into<PathBuf>,
        embedding_profile: SemanticModelProfile,
        embedding_model_path: impl Into<PathBuf>,
        ranking_mode: SemanticRankingMode,
        query: impl Into<String>,
    ) -> Result<Self> {
        let query = query.into();
        super::validate_query(&query)?;
        Ok(Self {
            identity,
            generation_root: generation_root.into(),
            embedding_profile,
            lexical_root: None,
            embedding_model_path: embedding_model_path.into(),
            reranker_profile: None,
            reranker_model_path: None,
            query,
            ranking_mode,
            commit_id: None,
            result_limit: None,
            protected_paths: Vec::new(),
        })
    }

    pub fn at_commit(mut self, commit_id: CommitId) -> Self {
        self.commit_id = Some(commit_id);
        self
    }

    pub fn with_result_limit(mut self, result_limit: usize) -> Result<Self> {
        if result_limit == 0 {
            return Err(crate::error::WorkVcsError::QueryInvalid(
                "semantic result limit must be greater than zero".to_owned(),
            ));
        }
        self.result_limit = Some(result_limit);
        Ok(self)
    }

    pub fn with_reranker_model(
        mut self,
        profile: SemanticModelProfile,
        path: impl Into<PathBuf>,
    ) -> Self {
        self.reranker_profile = Some(profile);
        self.reranker_model_path = Some(path.into());
        self
    }

    pub fn with_lexical_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.lexical_root = Some(path.into());
        self
    }

    pub fn with_protected_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.protected_paths.push(path.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SemanticPassageScore {
    pub point_id: u64,
    pub byte_start: usize,
    pub byte_end: usize,
    pub text: String,
    pub vector_score: Option<f32>,
    pub reranker_score: Option<f32>,
    pub winning_vector_range: bool,
    pub winning_reranker_range: bool,
    pub readback_verified: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SemanticSearchItem {
    pub source_type: SearchSourceType,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub entity_state_digest: Digest,
    pub statement_digest: Digest,
    pub status: String,
    pub record_kind: Option<String>,
    pub scope_json: String,
    pub provenance_json: Option<String>,
    pub canonical_statement: String,
    pub score: f64,
    pub vector_score: Option<f32>,
    pub reranker_score: Option<f32>,
    pub lexical_rank: Option<usize>,
    pub vector_rank: Option<usize>,
    pub reranker_rank: Option<usize>,
    pub passages: Vec<SemanticPassageScore>,
    pub readback_verified: bool,
    pub serialized_tokens: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct SemanticResourceObservations {
    pub elapsed_millis: u128,
    pub resident_memory_start_bytes: u64,
    pub resident_memory_end_bytes: u64,
    pub resident_memory_observed_max_bytes: u64,
    pub query_model_tokens: usize,
    pub passage_model_tokens: usize,
    pub reranker_pair_tokens: usize,
    pub query_window_count: usize,
    pub passage_count: usize,
    pub reranker_pair_count: usize,
    pub generation_bytes: u64,
    pub embedding_batch_size: usize,
    pub reranker_batch_size: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticDerivedStatus {
    Ready,
    RefreshRequired,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SemanticQueryResult {
    pub status: SemanticDerivedStatus,
    pub reason_code: Option<String>,
    pub reason: Option<String>,
    pub guidance: Option<String>,
    pub ranking_mode: SemanticRankingMode,
    pub query: String,
    pub source_identity: SearchSourceIdentity,
    pub temporal_scope: SearchTemporalScope,
    pub source_commit_id: CommitId,
    pub source_state_digest: Digest,
    pub generation_path: PathBuf,
    pub generation_manifest: SemanticGenerationManifest,
    pub embedding_model_path: PathBuf,
    pub reranker_model_path: Option<PathBuf>,
    pub generation_maintenance: super::SearchRefreshOutcome,
    pub result_limit: Option<usize>,
    pub candidates_considered: usize,
    pub matched_objects: usize,
    pub omitted_results: usize,
    pub serialized_item_tokens: usize,
    pub items: Vec<SemanticSearchItem>,
    pub canonical_readback_verified: bool,
    pub canonical_source_written: bool,
    pub derived_files_written: bool,
    pub observations: SemanticResourceObservations,
}

pub(crate) fn provision_model(
    options: &SemanticModelProvisionOptions,
) -> Result<SemanticModelProvisionResult> {
    model::provision_model(options)
}

pub(crate) fn query_semantic(
    source: &StoreConnection,
    source_path: &Path,
    options: &SemanticQueryOptions,
) -> Result<SemanticQueryResult> {
    generation::query_semantic(source, source_path, options)
}
