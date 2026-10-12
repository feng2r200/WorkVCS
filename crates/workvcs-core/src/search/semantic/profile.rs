use super::{SemanticModelKind, SemanticModelProfile};

pub(super) const MODEL_MANIFEST_FORMAT_VERSION: u32 = 1;
pub(super) const SEMANTIC_GENERATION_FORMAT_VERSION: u32 = 1;
pub(super) const SEMANTIC_RANKING_POLICY_VERSION: &str = "workvcs-semantic-ranking-v1";
pub(super) const SEMANTIC_WINDOW_POLICY_VERSION: &str = "workvcs-model-token-windows-v1-overlap-64";
pub(super) const VECTOR_NAME: &str = "workvcs-e5-large";
pub(super) const RRF_K: usize = 60;
pub(super) const E5_MAX_TOKENS: usize = 512;
pub(super) const E5_WINDOW_OVERLAP: usize = 64;
pub(super) const BGE_MAX_TOKENS: usize = 8192;

#[derive(Clone, Copy, Debug)]
pub(super) struct FileSpec {
    pub name: &'static str,
    pub source_name: &'static str,
    pub sha256: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ProfileSpec {
    pub profile: SemanticModelProfile,
    pub kind: SemanticModelKind,
    pub source: &'static str,
    pub revision: &'static str,
    pub files: &'static [FileSpec],
    pub max_tokens: usize,
    pub dimension: Option<usize>,
    pub query_prefix: Option<&'static str>,
    pub passage_prefix: Option<&'static str>,
    pub pooling: Option<&'static str>,
    pub output_key: &'static str,
}

const E5_FILES: &[FileSpec] = &[
    FileSpec {
        name: "model.onnx",
        source_name: "model.onnx",
        sha256: "1c09780c907c8a91a77a6ab1fd231f79e090d2907ca431223703dfebeed3d36c",
    },
    FileSpec {
        name: "model.onnx_data",
        source_name: "model.onnx_data",
        sha256: "0cf1883fee81c63819a44e2ba0efa51d4043d9759685a4ebebbde97e0623d15c",
    },
    FileSpec {
        name: "tokenizer.json",
        source_name: "tokenizer.json",
        sha256: "f59925fcb90c92b894cb93e51bb9b4a6105c5c249fe54ce1c704420ac39b81af",
    },
    FileSpec {
        name: "tokenizer_config.json",
        source_name: "tokenizer_config.json",
        sha256: "f90024142df07163e5e6c5b9a6ad7c8c68b22a9112af11e3db4559a9ff90f737",
    },
    FileSpec {
        name: "config.json",
        source_name: "config.json",
        sha256: "1de8c3be1f344c0eefa4962480a006f8639f416dfafaa95a770e3cf4bceae6a4",
    },
    FileSpec {
        name: "special_tokens_map.json",
        source_name: "special_tokens_map.json",
        sha256: "8c785abebea9ae3257b61681b4e6fd8365ceafde980c21970d001e834cf10835",
    },
];

const BGE_FILES: &[FileSpec] = &[
    FileSpec {
        name: "model.onnx",
        source_name: "onnx/model.onnx",
        sha256: "faae32b124a9d54afb7e89b5e9896e03c18a9552d56d1d6b273a709a83012486",
    },
    FileSpec {
        name: "model.onnx_data",
        source_name: "onnx/model.onnx_data",
        sha256: "f009aa6c6cf21986fd7e0021fa66b20ccce27abc6900a57c7109c8496811bcbe",
    },
    FileSpec {
        name: "tokenizer.json",
        source_name: "tokenizer.json",
        sha256: "8bf8afbfd11306bd872018c53bfdf2e160a56f8edbcf49933324404791c148d3",
    },
    FileSpec {
        name: "tokenizer_config.json",
        source_name: "tokenizer_config.json",
        sha256: "b87c8703482b0300d3da30e201519aa641f6a450f5eb5bf1e624afbf70c74d80",
    },
    FileSpec {
        name: "config.json",
        source_name: "config.json",
        sha256: "122e922dcfed6503c8721e6fe1daf090340c3d95ca7f3aa3a72730b321a51cfd",
    },
    FileSpec {
        name: "special_tokens_map.json",
        source_name: "special_tokens_map.json",
        sha256: "8c785abebea9ae3257b61681b4e6fd8365ceafde980c21970d001e834cf10835",
    },
];

pub(super) const E5_PROFILE: ProfileSpec = ProfileSpec {
    profile: SemanticModelProfile::MultilingualE5LargeFp32V1,
    kind: SemanticModelKind::Embedding,
    source: "Qdrant/multilingual-e5-large-onnx",
    revision: "ac6781cd1cf88b8306a536d7c9d18a5bd57cc14b",
    files: E5_FILES,
    max_tokens: E5_MAX_TOKENS,
    dimension: Some(1024),
    query_prefix: Some("query: "),
    passage_prefix: Some("passage: "),
    pooling: Some("mean"),
    output_key: "default_precedence",
};

pub(super) const BGE_PROFILE: ProfileSpec = ProfileSpec {
    profile: SemanticModelProfile::BgeRerankerV2M3Fp32V1,
    kind: SemanticModelKind::Reranker,
    source: "onnx-community/bge-reranker-v2-m3-ONNX",
    revision: "6f5ff65298512715a1e669753bc754d2bc8f367b",
    files: BGE_FILES,
    max_tokens: BGE_MAX_TOKENS,
    dimension: None,
    query_prefix: None,
    passage_prefix: None,
    pooling: None,
    output_key: "logits",
};

pub(super) fn profile_spec(profile: SemanticModelProfile) -> &'static ProfileSpec {
    match profile {
        SemanticModelProfile::MultilingualE5LargeFp32V1 => &E5_PROFILE,
        SemanticModelProfile::BgeRerankerV2M3Fp32V1 => &BGE_PROFILE,
    }
}
