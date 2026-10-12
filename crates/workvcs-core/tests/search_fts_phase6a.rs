use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, CanonicalValue, ContextPacketOptions, Digest, Engine, EntityVersionId,
    FindingRecordCorrectionOptions, KnowledgeCreateOptions, KnowledgeStatus,
    KnowledgeTransitionOptions, MergeContinueOptions, MergeFreezeResolutionsOptions,
    MergeResolveOptions, MergeStartOptions, ProjectRefId, RecordCreateOptions,
    RunnableTasksOptions, SearchDerivedStatus, SearchIndexManifest, SearchQueryOptions,
    SearchRangeRole, SearchRefreshOptions, SearchRefreshOutcome, SearchSourceIdentity,
    SearchSourceType, SearchTemporalScope, SessionStartOptions, StoreId, StoreInitOptions,
    TaskCreateOptions, WorkspaceId, WorkspaceInfo, WorkspaceInitOptions, search_item_context_json,
    search_o200k_ordinary_token_count,
};

struct Fixture {
    _tempdir: TempDir,
    source_path: PathBuf,
    index_path: PathBuf,
    project_ref_id: ProjectRefId,
    workspace: WorkspaceInfo,
    store_id: StoreId,
}

impl Fixture {
    fn new(name: &str) -> (Self, Engine) {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let source_path = tempdir.path().join("workvcs.sqlite");
        let index_path = tempdir.path().join("search.sqlite");
        let mut engine = Engine::init(
            &source_path,
            StoreInitOptions::new(format!("{name}-store")).expect("store options"),
        )
        .expect("init engine");
        let store_id = engine.store_info().expect("store info").store_id;
        let workspace = engine
            .create_workspace(
                WorkspaceInitOptions::new(format!("{name}-workspace")).expect("workspace options"),
            )
            .expect("create workspace");
        (
            Self {
                _tempdir: tempdir,
                source_path,
                index_path,
                project_ref_id: ProjectRefId::new_v7(),
                workspace,
                store_id,
            },
            engine,
        )
    }

    fn identity(&self) -> SearchSourceIdentity {
        SearchSourceIdentity::new(
            self.project_ref_id,
            self.store_id,
            self.workspace.workspace_id,
            self.workspace.initial_branch_id,
        )
    }

    fn refresh_options(&self) -> SearchRefreshOptions {
        SearchRefreshOptions::new(self.identity(), &self.index_path)
    }

    fn query_options(&self, query: &str) -> SearchQueryOptions {
        SearchQueryOptions::new(self.identity(), &self.index_path, query).expect("query options")
    }
}

fn update_index(path: &Path, sql: &str, values: impl rusqlite::Params) {
    let connection = Connection::open(path).expect("open derived index for corruption fixture");
    connection
        .execute(sql, values)
        .expect("mutate derived index fixture");
}

fn manifest_digest(manifest: &SearchIndexManifest) -> Digest {
    let payload = format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        manifest.index_format_version,
        manifest.tokenizer_version,
        manifest.snippet_policy_version,
        manifest.token_count_encoding,
        manifest.identity.project_ref_id,
        manifest.identity.store_id,
        manifest.identity.workspace_id,
        manifest.identity.branch_id,
        manifest.temporal_scope.as_str(),
        manifest.source_commit_id,
        manifest.source_state_digest,
        manifest.object_version_basis_digest,
        manifest.object_count,
        manifest.snippet_count,
        manifest.created_at_us,
    );
    Digest::domain_separated("workvcs-search-index-manifest-v2", payload.as_bytes())
}

fn merge_detail(reason: &str) -> CanonicalValue {
    CanonicalValue::object(vec![(
        "reason".to_owned(),
        CanonicalValue::String(reason.to_owned()),
    )])
    .expect("merge detail")
}

#[test]
fn refresh_and_search_return_verified_chinese_and_technical_evidence_without_source_writes() {
    let (fixture, mut engine) = Fixture::new("search-positive");
    let record = engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "主索引策略使用 ProjectRef-v2 与 state_digest，中文检索必须回读同一 canonical statement。",
            )
            .expect("record options"),
        )
        .expect("create record");
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                fixture.workspace.initial_branch_id,
                record.commit_id,
                "ReplayCache 批量读取避免每个 hit 重复 record_at 回放。",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                fixture.workspace.initial_branch_id,
                knowledge.commit_id,
                "task_only_marker must stay in native runnable and context projections",
            )
            .expect("task options"),
        )
        .expect("create task");
    let session = engine
        .start_session(
            SessionStartOptions::new(
                fixture.workspace.workspace_id,
                fixture.workspace.initial_branch_id,
            )
            .expect("session options"),
        )
        .expect("start session");
    let context_before = engine
        .context_packet(ContextPacketOptions::new(session.session_id))
        .expect("context before search");
    let runnable_before = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("runnable before search");
    assert!(
        runnable_before
            .candidates
            .iter()
            .any(|candidate| candidate.task.task_entity_id == task.task_entity_id)
    );
    drop(engine);

    let source_before = fs::read(&fixture.source_path).expect("source before");
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    let refreshed = readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("refresh search index");
    assert_eq!(refreshed.outcome, SearchRefreshOutcome::Refreshed);
    assert_eq!(refreshed.records_indexed, 1);
    assert_eq!(refreshed.knowledge_indexed, 1);
    assert!(!refreshed.canonical_source_written);
    assert_eq!(refreshed.manifest.object_count, 2);
    assert_eq!(
        refreshed.manifest.temporal_scope,
        SearchTemporalScope::Current
    );

    let record_result = readonly
        .search(fixture.query_options("state_digest"))
        .expect("search record");
    assert_eq!(record_result.status, SearchDerivedStatus::Ready);
    assert!(record_result.canonical_readback_verified);
    assert_eq!(record_result.items.len(), 1);
    let item = &record_result.items[0];
    assert_eq!(item.source_type, SearchSourceType::Record);
    assert_eq!(item.entity_id, record.record_entity_id);
    assert_eq!(item.entity_version_id, record.record_entity_version_id);
    assert_eq!(item.entity_state_digest, record.record_state_digest);
    assert_eq!(item.ranges.len(), 1);
    let range = &item.ranges[0];
    assert_eq!(
        range.role,
        SearchRangeRole::QueryMatchAndDocumentLeadContext
    );
    assert!(range.snippet.contains("ProjectRef-v2"));
    assert!(range.snippet.contains("state_digest"));
    assert_eq!(range.snippet_byte_start, 0);
    assert_eq!(range.snippet_byte_end, range.snippet.len());
    assert!(range.readback_verified);
    assert!(item.readback_verified);
    let item_context_json = search_item_context_json(item).expect("render complete search item");
    assert_eq!(
        item.serialized_tokens,
        search_o200k_ordinary_token_count(&item_context_json),
        "token observation must cover the exact complete structured item"
    );
    assert_eq!(
        item_context_json,
        serde_json::to_string(item).expect("serialize complete item")
    );
    assert_eq!(record_result.serialized_item_tokens, item.serialized_tokens);
    assert_eq!(item.canonical_statement, range.snippet);

    let knowledge_result = readonly
        .search(fixture.query_options("ReplayCache record_at"))
        .expect("search knowledge");
    assert_eq!(knowledge_result.items.len(), 1);
    assert_eq!(
        knowledge_result.items[0].source_type,
        SearchSourceType::Knowledge
    );
    assert_eq!(
        knowledge_result.items[0].entity_id,
        knowledge.knowledge_entity_id
    );
    let task_result = readonly
        .search(fixture.query_options("task_only_marker"))
        .expect("task exclusion query");
    assert!(
        task_result.items.is_empty(),
        "Task must not enter the FTS index"
    );
    assert_eq!(
        readonly
            .context_packet(ContextPacketOptions::new(session.session_id))
            .expect("context after search"),
        context_before,
        "derived search must not alter native context semantics"
    );
    assert_eq!(
        readonly
            .runnable_tasks(RunnableTasksOptions::new(session.session_id))
            .expect("runnable after search"),
        runnable_before,
        "derived scores must not alter Task runnable projection"
    );

    let reused = readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("reuse identical index");
    assert_eq!(reused.outcome, SearchRefreshOutcome::Reused);
    assert_eq!(
        fs::read(&fixture.source_path).expect("source after"),
        source_before,
        "refresh and query must not write the canonical Store"
    );

    let missing_path = fixture._tempdir.path().join("missing.sqlite");
    let missing = readonly
        .search(SearchQueryOptions::new(fixture.identity(), missing_path, "state_digest").unwrap())
        .expect("missing derived index is diagnostic");
    assert_eq!(missing.status, SearchDerivedStatus::RefreshRequired);
    assert_eq!(missing.reason_code.as_deref(), Some("missing_index"));
    assert!(
        missing
            .guidance
            .as_deref()
            .unwrap()
            .contains("native context")
    );

    let unlimited = readonly
        .search(fixture.query_options("state_digest"))
        .expect("unlimited search");
    assert_eq!(unlimited.result_limit, None);
    assert_eq!(unlimited.omitted_results, 0);
    assert_eq!(
        unlimited.items[0].canonical_statement,
        item.canonical_statement
    );
    assert!(SearchQueryOptions::new(fixture.identity(), &fixture.index_path, " ").is_err());
    assert!(
        fixture
            .query_options("state_digest")
            .with_result_limit(0)
            .is_err()
    );

    update_index(
        &fixture.index_path,
        "UPDATE passage_fts
         SET terms = 'tampered_nonmatching_terms'
         WHERE snippet_id IN (
             SELECT s.snippet_id
             FROM snippet s JOIN document d ON d.document_key = s.document_key
             WHERE d.source_type = 'knowledge'
         )",
        [],
    );
    let damaged_nonmatching_row = readonly
        .search(fixture.query_options("state_digest"))
        .expect("nonmatching FTS damage must be diagnostic");
    assert_eq!(
        damaged_nonmatching_row.status,
        SearchDerivedStatus::RefreshRequired
    );
    assert_eq!(
        damaged_nonmatching_row.reason_code.as_deref(),
        Some("damaged_index")
    );
    assert!(damaged_nonmatching_row.items.is_empty());
    assert!(
        readonly
            .refresh_search_index(fixture.refresh_options())
            .expect_err("damaged existing generation must not be replaced")
            .to_string()
            .contains("select a new index path")
    );
}

#[test]
fn search_aggregates_all_query_regions_with_lead_context_and_preserves_adjacent_denial() {
    let (fixture, mut engine) = Fixture::new("search-multi-range");
    let multi_statement = format!(
        "适用范围：以下事实只描述隔离测试夹具，不授予任何生产权限。{} first_fact_signal 第一项事实成立。{} second_fact_signal 第二项事实也成立。{} third_fact_signal 第三项事实也成立。",
        "前导中性说明。".repeat(120),
        "中段中性说明。".repeat(160),
        "后段中性说明。".repeat(160),
    );
    let multi = engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                multi_statement.clone(),
            )
            .expect("multi-range record options"),
        )
        .expect("multi-range record");
    let denial_statement = format!(
        "适用范围：引用内容仅用于说明语境，不能视为当前结论。{}有人引用错误陈述 quoted_false_signal：WorkVCS 未绑定，因此只能只读。紧接着的限定与否定：该陈述已经失效，当前绑定必须以核验后的 ProjectRef 为准。",
        "独立背景材料。".repeat(120),
    );
    let denial = engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                multi.commit_id,
                denial_statement.clone(),
            )
            .expect("denial record options"),
        )
        .expect("denial record");
    drop(engine);

    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("refresh multi-range index");
    let multi_result = readonly
        .search(fixture.query_options("first_fact_signal second_fact_signal third_fact_signal"))
        .expect("search all separated facts");
    let multi_item = multi_result
        .items
        .iter()
        .find(|item| item.entity_id == multi.record_entity_id)
        .expect("multi-fact record result");
    assert!(multi_item.ranges.len() >= 4);
    assert!(
        multi_item
            .ranges
            .iter()
            .any(|range| range.role == SearchRangeRole::DocumentLeadContext
                && range.snippet.contains("适用范围"))
    );
    let query_ranges = multi_item
        .ranges
        .iter()
        .filter(|range| {
            matches!(
                range.role,
                SearchRangeRole::QueryMatch | SearchRangeRole::QueryMatchAndDocumentLeadContext
            )
        })
        .collect::<Vec<_>>();
    assert!(query_ranges.len() >= 3);
    assert!(
        query_ranges
            .iter()
            .any(|range| range.snippet.contains("first_fact_signal"))
    );
    assert!(
        query_ranges
            .iter()
            .any(|range| range.snippet.contains("second_fact_signal"))
    );
    assert!(
        query_ranges
            .iter()
            .any(|range| range.snippet.contains("third_fact_signal"))
    );
    for range in &multi_item.ranges {
        assert!(multi_statement.is_char_boundary(range.snippet_byte_start));
        assert!(multi_statement.is_char_boundary(range.snippet_byte_end));
        assert_eq!(
            &multi_statement[range.snippet_byte_start..range.snippet_byte_end],
            range.snippet
        );
        assert!(range.readback_verified);
    }

    assert_eq!(multi_item.canonical_statement, multi_statement);

    let denial_result = readonly
        .search(fixture.query_options("quoted_false_signal"))
        .expect("search quoted statement");
    let denial_item = denial_result
        .items
        .iter()
        .find(|item| item.entity_id == denial.record_entity_id)
        .expect("quoted statement record result");
    assert_eq!(denial_item.canonical_statement, denial_statement);
    assert!(
        denial_item
            .ranges
            .iter()
            .any(|range| range.role == SearchRangeRole::DocumentLeadContext
                && range.snippet.contains("不能视为当前结论"))
    );
    assert!(denial_item.ranges.iter().any(|range| {
        matches!(
            range.role,
            SearchRangeRole::QueryMatch | SearchRangeRole::QueryMatchAndDocumentLeadContext
        ) && range.snippet.contains("quoted_false_signal")
            && range.snippet.contains("该陈述已经失效")
    }));
    for range in &denial_item.ranges {
        assert_eq!(
            &denial_statement[range.snippet_byte_start..range.snippet_byte_end],
            range.snippet
        );
    }
}

#[test]
fn search_defaults_to_all_matches_and_only_explicit_limit_omits_results() {
    let (fixture, mut engine) = Fixture::new("search-unlimited-default");
    let mut head = fixture.workspace.genesis_commit_id;
    for index in 0..25 {
        let record = engine
            .create_record(
                RecordCreateOptions::finding(
                    fixture.workspace.initial_branch_id,
                    head,
                    format!("unlimited_default_marker isolated synthetic record {index:02}"),
                )
                .expect("record options"),
            )
            .expect("create matching record");
        head = record.commit_id;
    }
    drop(engine);

    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("refresh unlimited index");

    let unlimited = readonly
        .search(fixture.query_options("unlimited_default_marker"))
        .expect("default unlimited search");
    assert_eq!(unlimited.result_limit, None);
    assert_eq!(unlimited.matched_objects, 25);
    assert_eq!(unlimited.items.len(), 25);
    assert_eq!(unlimited.omitted_results, 0);
    assert_eq!(
        unlimited.serialized_item_tokens,
        unlimited
            .items
            .iter()
            .map(|item| item.serialized_tokens)
            .sum::<usize>()
    );

    let limited = readonly
        .search(
            fixture
                .query_options("unlimited_default_marker")
                .with_result_limit(5)
                .expect("explicit result limit"),
        )
        .expect("explicitly limited search");
    assert_eq!(limited.result_limit, Some(5));
    assert_eq!(limited.matched_objects, 25);
    assert_eq!(limited.items.len(), 5);
    assert_eq!(limited.omitted_results, 20);
    assert!(limited.serialized_item_tokens > 0);

    let long_query = "unlimited_default_marker ".repeat(300);
    assert!(long_query.len() > 4096);
    let long_result = readonly
        .search(fixture.query_options(&long_query))
        .expect("long query is not rejected by an artificial byte cap");
    assert_eq!(long_result.items.len(), 25);
    assert_eq!(long_result.omitted_results, 0);
}

#[test]
fn current_and_historical_scopes_enforce_lifecycle_branch_and_identity_boundaries() {
    let (fixture, mut engine) = Fixture::new("search-temporal");
    let prior = engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "obsolete_only_marker 旧发现只应在显式历史检索中出现",
            )
            .unwrap(),
        )
        .expect("prior finding");
    let replacement = engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                prior.commit_id,
                "replacement_only_marker 当前发现保持有效",
            )
            .unwrap(),
        )
        .expect("replacement finding");
    let correction = engine
        .correct_finding_record(
            FindingRecordCorrectionOptions::supersede(
                fixture.workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "validated replacement",
            )
            .unwrap(),
        )
        .expect("supersede prior finding");
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                fixture.workspace.initial_branch_id,
                correction.commit_id,
                "invalidated_knowledge_marker 旧知识",
            )
            .unwrap(),
        )
        .expect("knowledge");
    let terminal = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::invalidate(
                fixture.workspace.initial_branch_id,
                knowledge.commit_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
                "canonical evidence disproved it",
            )
            .unwrap(),
        )
        .expect("invalidate knowledge");
    assert_eq!(terminal.state.status, KnowledgeStatus::Invalidated);

    let fork = engine
        .fork_branch(
            BranchForkOptions::from_commit(terminal.commit_id, "isolated-search-branch").unwrap(),
        )
        .expect("fork branch");
    let branch_record = engine
        .create_record(
            RecordCreateOptions::finding(
                fork.branch_id,
                fork.head_commit_id,
                "fork_only_marker branch local evidence",
            )
            .unwrap(),
        )
        .expect("fork record");
    drop(engine);

    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("refresh current");
    let obsolete_current = readonly
        .search(fixture.query_options("obsolete_only_marker"))
        .expect("current old finding query");
    assert!(obsolete_current.items.is_empty());
    let knowledge_current = readonly
        .search(fixture.query_options("invalidated_knowledge_marker"))
        .expect("current old knowledge query");
    assert!(knowledge_current.items.is_empty());
    let replacement_current = readonly
        .search(fixture.query_options("replacement_only_marker"))
        .expect("current replacement query");
    assert_eq!(replacement_current.items.len(), 1);

    let historical_path = fixture._tempdir.path().join("historical.sqlite");
    readonly
        .refresh_search_index(
            SearchRefreshOptions::new(fixture.identity(), &historical_path)
                .at_commit(terminal.commit_id),
        )
        .expect("refresh historical");
    let historical = readonly
        .search(
            SearchQueryOptions::new(
                fixture.identity(),
                &historical_path,
                "obsolete_only_marker invalidated_knowledge_marker",
            )
            .unwrap()
            .at_commit(terminal.commit_id),
        )
        .expect("historical query");
    assert_eq!(historical.temporal_scope, SearchTemporalScope::Historical);
    assert_eq!(historical.items.len(), 2);
    assert!(
        historical
            .items
            .iter()
            .any(|item| item.status == "superseded")
    );
    assert!(
        historical
            .items
            .iter()
            .any(|item| item.status == "invalidated")
    );

    let unreachable = readonly
        .search(
            fixture
                .query_options("fork_only_marker")
                .at_commit(branch_record.commit_id),
        )
        .expect_err("other branch commit must not be accepted as main history");
    assert!(unreachable.to_string().contains("not reachable"));

    let wrong_project = SearchSourceIdentity::new(
        ProjectRefId::new_v7(),
        fixture.store_id,
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
    );
    let mismatch = readonly
        .search(
            SearchQueryOptions::new(
                wrong_project,
                &fixture.index_path,
                "replacement_only_marker",
            )
            .unwrap(),
        )
        .expect("identity mismatch is refresh diagnostic");
    assert_eq!(
        mismatch.reason_code.as_deref(),
        Some("source_identity_mismatch")
    );
    let before_failed_refresh = fs::read(&fixture.index_path).expect("index before failed refresh");
    assert!(
        readonly
            .refresh_search_index(SearchRefreshOptions::new(
                wrong_project,
                &fixture.index_path
            ))
            .is_err()
    );
    assert_eq!(
        fs::read(&fixture.index_path).expect("index after failed refresh"),
        before_failed_refresh,
        "failed refresh must preserve the last published index"
    );
    let still_ready = readonly
        .search(fixture.query_options("replacement_only_marker"))
        .expect("old publication remains queryable");
    assert_eq!(still_ready.status, SearchDerivedStatus::Ready);

    let wrong_store = SearchSourceIdentity::new(
        fixture.project_ref_id,
        StoreId::new_v7(),
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
    );
    assert!(
        readonly
            .search(SearchQueryOptions::new(wrong_store, &fixture.index_path, "marker").unwrap())
            .unwrap_err()
            .to_string()
            .contains("does not match routed Store")
    );
    let wrong_workspace = SearchSourceIdentity::new(
        fixture.project_ref_id,
        fixture.store_id,
        WorkspaceId::new_v7(),
        fixture.workspace.initial_branch_id,
    );
    assert!(
        readonly
            .search(
                SearchQueryOptions::new(wrong_workspace, &fixture.index_path, "marker").unwrap()
            )
            .unwrap_err()
            .to_string()
            .contains("not routed workspace")
    );
    let fork_identity = SearchSourceIdentity::new(
        fixture.project_ref_id,
        fixture.store_id,
        fixture.workspace.workspace_id,
        fork.branch_id,
    );
    let branch_mismatch = readonly
        .search(
            SearchQueryOptions::new(fork_identity, &fixture.index_path, "fork_only_marker")
                .unwrap(),
        )
        .expect("branch mismatch diagnostic");
    assert_eq!(
        branch_mismatch.reason_code.as_deref(),
        Some("source_identity_mismatch")
    );

    drop(readonly);
    let mut writer = Engine::open(&fixture.source_path).expect("writer");
    writer
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                terminal.commit_id,
                "head_advanced_marker newly delivered state",
            )
            .unwrap(),
        )
        .expect("advance selected branch");
    drop(writer);
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly updated source");
    let stale = readonly
        .search(fixture.query_options("replacement_only_marker"))
        .expect("stale query diagnostic");
    assert_eq!(stale.status, SearchDerivedStatus::RefreshRequired);
    assert_eq!(stale.reason_code.as_deref(), Some("stale_index"));
    assert!(stale.items.is_empty());
}

#[test]
fn historical_scope_accepts_secondary_merge_ancestors_and_rejects_other_dags() {
    let (fixture, mut engine) = Fixture::new("search-merge-dag");
    let base = engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "merge_base_marker shared source",
            )
            .unwrap(),
        )
        .expect("base record");
    let source_branch = engine
        .fork_branch(
            BranchForkOptions::from_branch(fixture.workspace.initial_branch_id, "merge-source")
                .unwrap(),
        )
        .expect("source branch");
    let target = engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                base.commit_id,
                "target_only_marker target branch",
            )
            .unwrap(),
        )
        .expect("target record");
    let source = engine
        .create_record(
            RecordCreateOptions::finding(
                source_branch.branch_id,
                base.commit_id,
                "secondary_parent_marker source branch evidence",
            )
            .unwrap(),
        )
        .expect("source record");

    let before_merge_path = fixture._tempdir.path().join("before-merge.sqlite");
    assert!(
        engine
            .refresh_search_index(
                SearchRefreshOptions::new(fixture.identity(), &before_merge_path)
                    .at_commit(source.commit_id),
            )
            .expect_err("unmerged source commit is not selected-branch history")
            .to_string()
            .contains("not reachable")
    );

    let merge = engine
        .start_merge(MergeStartOptions::new(
            fixture.workspace.initial_branch_id,
            source_branch.branch_id,
        ))
        .expect("start merge");
    let attempt = engine.merge_attempt(merge.merge_id).expect("merge attempt");
    for item in attempt.items {
        engine
            .resolve_merge_item(
                MergeResolveOptions::theirs(item.merge_item_id)
                    .expect("theirs resolution")
                    .with_rationale(merge_detail("include source branch"))
                    .expect("resolution rationale"),
            )
            .expect("resolve merge item");
    }
    engine
        .freeze_merge_resolutions(MergeFreezeResolutionsOptions::new(merge.merge_id))
        .expect("freeze merge");
    let merged = engine
        .continue_merge(
            MergeContinueOptions::new(merge.merge_id)
                .expect("continue options")
                .with_detail(merge_detail("complete merge"))
                .expect("merge detail"),
        )
        .expect("continue merge");
    assert_eq!(
        engine
            .commit(merged.result_commit_id)
            .expect("merge commit")
            .parents[1]
            .parent_commit_id,
        source.commit_id
    );

    let unmerged_branch = engine
        .fork_branch(BranchForkOptions::from_commit(target.commit_id, "never-merged").unwrap())
        .expect("unmerged branch");
    let unmerged = engine
        .create_record(
            RecordCreateOptions::finding(
                unmerged_branch.branch_id,
                unmerged_branch.head_commit_id,
                "never_merged_marker unrelated branch",
            )
            .unwrap(),
        )
        .expect("unmerged record");
    let other_workspace = engine
        .create_workspace(WorkspaceInitOptions::new("other-workspace").unwrap())
        .expect("other workspace");
    let cross_workspace = engine
        .create_record(
            RecordCreateOptions::finding(
                other_workspace.initial_branch_id,
                other_workspace.genesis_commit_id,
                "cross_workspace_marker",
            )
            .unwrap(),
        )
        .expect("cross workspace record");
    drop(engine);

    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    let merged_history_path = fixture._tempdir.path().join("merged-history.sqlite");
    readonly
        .refresh_search_index(
            SearchRefreshOptions::new(fixture.identity(), &merged_history_path)
                .at_commit(source.commit_id),
        )
        .expect("secondary merge parent is a valid historical source");
    let result = readonly
        .search(
            SearchQueryOptions::new(
                fixture.identity(),
                &merged_history_path,
                "secondary_parent_marker",
            )
            .unwrap()
            .at_commit(source.commit_id),
        )
        .expect("query secondary parent history");
    assert_eq!(result.status, SearchDerivedStatus::Ready);
    assert_eq!(result.items[0].entity_id, source.record_entity_id);

    assert!(
        readonly
            .refresh_search_index(
                SearchRefreshOptions::new(
                    fixture.identity(),
                    fixture._tempdir.path().join("unmerged.sqlite"),
                )
                .at_commit(unmerged.commit_id),
            )
            .expect_err("unmerged DAG commit must be rejected")
            .to_string()
            .contains("not reachable")
    );
    assert!(
        readonly
            .refresh_search_index(
                SearchRefreshOptions::new(
                    fixture.identity(),
                    fixture._tempdir.path().join("cross-workspace.sqlite"),
                )
                .at_commit(cross_workspace.commit_id),
            )
            .expect_err("cross-workspace commit must be rejected")
            .to_string()
            .contains("belongs to workspace")
    );
}

#[test]
fn canonical_readback_rejects_wrong_version_digest_and_snippet_identity() {
    let (fixture, mut engine) = Fixture::new("search-readback");
    engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "中文开头 canonical_guard_marker verifies exact statement bytes",
            )
            .unwrap(),
        )
        .expect("record");
    drop(engine);
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("initial refresh");
    let pristine = fs::read(&fixture.index_path).expect("pristine index bytes");

    update_index(
        &fixture.index_path,
        "UPDATE document SET entity_version_id = ?1",
        params![EntityVersionId::new_v7().to_string()],
    );
    let wrong_version = readonly
        .search(fixture.query_options("canonical_guard_marker"))
        .expect("wrong version diagnostic");
    assert_eq!(wrong_version.reason_code.as_deref(), Some("damaged_index"));

    let wrong_digest_path = fixture._tempdir.path().join("wrong-digest.sqlite");
    fs::write(&wrong_digest_path, &pristine).expect("copy pristine wrong digest index");
    update_index(
        &wrong_digest_path,
        "UPDATE document SET entity_state_digest = ?1",
        params![Digest::raw(b"wrong state").to_string()],
    );
    let wrong_digest = readonly
        .search(
            SearchQueryOptions::new(
                fixture.identity(),
                &wrong_digest_path,
                "canonical_guard_marker",
            )
            .unwrap(),
        )
        .expect("wrong digest diagnostic");
    assert_eq!(wrong_digest.reason_code.as_deref(), Some("damaged_index"));

    let wrong_range_path = fixture._tempdir.path().join("wrong-range.sqlite");
    fs::write(&wrong_range_path, &pristine).expect("copy pristine wrong range index");
    update_index(&wrong_range_path, "UPDATE snippet SET byte_start = 1", []);
    let wrong_range = readonly
        .search(
            SearchQueryOptions::new(
                fixture.identity(),
                &wrong_range_path,
                "canonical_guard_marker",
            )
            .unwrap(),
        )
        .expect("wrong byte range diagnostic");
    assert_eq!(wrong_range.reason_code.as_deref(), Some("damaged_index"));

    let wrong_content_path = fixture._tempdir.path().join("wrong-content.sqlite");
    fs::write(&wrong_content_path, &pristine).expect("copy pristine wrong content index");
    update_index(
        &wrong_content_path,
        "UPDATE snippet SET snippet = 'tampered snippet', snippet_digest = ?1",
        params![
            Digest::domain_separated("workvcs-search-statement-v1", b"tampered snippet")
                .to_string()
        ],
    );
    let wrong_content = readonly
        .search(
            SearchQueryOptions::new(
                fixture.identity(),
                &wrong_content_path,
                "canonical_guard_marker",
            )
            .unwrap(),
        )
        .expect("wrong snippet content diagnostic");
    assert_eq!(wrong_content.reason_code.as_deref(), Some("damaged_index"));
}

#[test]
fn complete_expected_snippet_rows_reject_self_consistent_duplicate_window_false_empty() {
    let (fixture, mut engine) = Fixture::new("search-snippet-row-completeness");
    let statement = format!(
        "validation_anchor_marker lead. {} unique_window_marker final evidence.",
        "neutral filler content. ".repeat(260)
    );
    engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                statement,
            )
            .unwrap(),
        )
        .expect("record");
    drop(engine);
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("refresh index");

    let connection = Connection::open(&fixture.index_path).expect("open derived index");
    let unique_id = connection
        .query_row(
            "SELECT snippet_id FROM snippet WHERE snippet LIKE '%unique_window_marker%'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("unique snippet id");
    let replacement = connection
        .query_row(
            "SELECT s.byte_start, s.byte_end, s.snippet, s.snippet_digest, f.terms
             FROM snippet s JOIN passage_fts f ON f.snippet_id = s.snippet_id
             WHERE s.snippet_id <> ?1 ORDER BY s.snippet_id LIMIT 1",
            params![unique_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .expect("replacement canonical window");
    connection
        .execute(
            "UPDATE snippet
             SET byte_start=?1, byte_end=?2, snippet=?3, snippet_digest=?4
             WHERE snippet_id=?5",
            params![
                replacement.0,
                replacement.1,
                replacement.2,
                replacement.3,
                unique_id
            ],
        )
        .expect("replace snippet metadata with another valid window");
    connection
        .execute(
            "UPDATE passage_fts SET terms=?1 WHERE snippet_id=?2",
            params![replacement.4, unique_id],
        )
        .expect("replace FTS content with matching duplicate terms");
    let direct_false_empty = connection
        .query_row(
            "SELECT count(*) FROM passage_fts WHERE passage_fts MATCH 'unique_window_marker'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("direct false-empty query");
    assert_eq!(
        direct_false_empty, 0,
        "fixture must reproduce the false empty"
    );
    drop(connection);

    for query in ["unique_window_marker", "validation_anchor_marker"] {
        let result = readonly
            .search(fixture.query_options(query))
            .expect("damaged index is a derived diagnostic");
        assert_eq!(result.status, SearchDerivedStatus::RefreshRequired);
        assert_eq!(result.reason_code.as_deref(), Some("damaged_index"));
        assert!(result.items.is_empty());
    }
}

#[test]
fn fts5_engine_integrity_rejects_shadow_posting_damage_even_for_nonmatching_query() {
    let (fixture, mut engine) = Fixture::new("search-fts-shadow-integrity");
    engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "unique_posting_marker canonical FTS content row",
            )
            .unwrap(),
        )
        .expect("record");
    drop(engine);
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("refresh index");

    let connection = Connection::open(&fixture.index_path).expect("open derived index");
    let terms_before = connection
        .query_row(
            "SELECT terms FROM passage_fts WHERE snippet_id=1",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("FTS content terms");
    assert!(terms_before.contains("unique_posting_marker"));
    connection
        .execute(
            "UPDATE passage_fts_data
             SET block=CAST(replace(CAST(block AS TEXT), 'unique', 'xxxxxx') AS BLOB)
             WHERE id > 10",
            [],
        )
        .expect("mutate only FTS segment posting bytes");
    assert_ne!(
        connection
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .expect("ordinary SQLite integrity check result"),
        "ok",
        "the bundled SQLite build also exposes this posting corruption through its virtual-table integrity hook"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT terms FROM passage_fts WHERE snippet_id=1",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("FTS content row remains"),
        terms_before,
        "the content row must remain unchanged while postings are damaged"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM passage_fts WHERE passage_fts MATCH 'unique'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("direct corrupted posting query"),
        0,
        "fixture must reproduce a false empty while the FTS content row remains present"
    );
    drop(connection);

    for query in ["unique_posting_marker", "definitely_nonmatching_marker"] {
        let result = readonly
            .search(fixture.query_options(query))
            .expect("posting damage is a derived diagnostic");
        assert_eq!(result.status, SearchDerivedStatus::RefreshRequired);
        assert_eq!(result.reason_code.as_deref(), Some("damaged_index"));
        assert!(result.items.is_empty());
    }
}

#[test]
fn unsafe_alias_unknown_and_damaged_destinations_are_rejected_without_overwrite() {
    let (fixture, mut engine) = Fixture::new("search-path-safety");
    engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "path_safety_marker source bytes are protected",
            )
            .unwrap(),
        )
        .expect("record");
    drop(engine);
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    let source_before = fs::read(&fixture.source_path).expect("source before");

    assert!(
        readonly
            .refresh_search_index(SearchRefreshOptions::new(
                fixture.identity(),
                &fixture.source_path,
            ))
            .is_err()
    );

    let unknown = fixture._tempdir.path().join("unknown.bin");
    fs::write(&unknown, b"user-owned bytes").expect("unknown file");
    assert!(
        readonly
            .refresh_search_index(SearchRefreshOptions::new(fixture.identity(), &unknown))
            .is_err()
    );
    assert_eq!(fs::read(&unknown).unwrap(), b"user-owned bytes");
    let damaged = readonly
        .search(
            SearchQueryOptions::new(fixture.identity(), &unknown, "path_safety_marker").unwrap(),
        )
        .expect("damaged index diagnostic");
    assert_eq!(damaged.status, SearchDerivedStatus::RefreshRequired);
    assert_eq!(damaged.reason_code.as_deref(), Some("damaged_index"));

    let protected = fixture._tempdir.path().join("registry.sqlite");
    fs::write(&protected, b"registry bytes").expect("protected file");
    assert!(
        readonly
            .refresh_search_index(
                SearchRefreshOptions::new(fixture.identity(), &protected)
                    .with_protected_path(&protected),
            )
            .is_err()
    );
    assert_eq!(fs::read(&protected).unwrap(), b"registry bytes");

    for suffix in ["-journal", "-wal", "-shm", "-JOURNAL", "-WAL", "-SHM"] {
        let mut sidecar_name = fixture.source_path.file_name().unwrap().to_os_string();
        sidecar_name.push(suffix);
        let sidecar = fixture.source_path.with_file_name(sidecar_name);
        assert!(!sidecar.exists(), "sidecar fixture must start absent");
        assert!(
            readonly
                .refresh_search_index(SearchRefreshOptions::new(fixture.identity(), &sidecar))
                .expect_err("reserved Store sidecar path must reject refresh")
                .to_string()
                .contains("reserved canonical Store SQLite sidecar")
        );
        let query = readonly
            .search(
                SearchQueryOptions::new(fixture.identity(), &sidecar, "path_safety_marker")
                    .unwrap(),
            )
            .expect("reserved Store sidecar query is diagnostic");
        assert_eq!(query.status, SearchDerivedStatus::RefreshRequired);
        assert_eq!(query.reason_code.as_deref(), Some("unsafe_index_alias"));
        assert!(
            !sidecar.exists(),
            "path guard must not create a Store sidecar"
        );
    }

    #[cfg(unix)]
    {
        let symlink_path = fixture._tempdir.path().join("source-symlink.sqlite");
        std::os::unix::fs::symlink(&fixture.source_path, &symlink_path).expect("source symlink");
        assert!(
            readonly
                .refresh_search_index(SearchRefreshOptions::new(fixture.identity(), &symlink_path,))
                .is_err()
        );
        fs::remove_file(&symlink_path).expect("remove symlink");

        let hardlink_path = fixture._tempdir.path().join("source-hardlink.sqlite");
        fs::hard_link(&fixture.source_path, &hardlink_path).expect("source hard link");
        assert!(
            readonly
                .refresh_search_index(SearchRefreshOptions::new(
                    fixture.identity(),
                    &hardlink_path,
                ))
                .is_err()
        );
        fs::remove_file(&hardlink_path).expect("remove hard link");
    }
    assert_eq!(fs::read(&fixture.source_path).unwrap(), source_before);
}

#[test]
fn incompatible_manifest_is_a_refresh_diagnostic_instead_of_empty_success() {
    let (fixture, mut engine) = Fixture::new("search-incompatible");
    engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "incompatible_manifest_marker remains canonical",
            )
            .unwrap(),
        )
        .expect("record");
    drop(engine);
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly source");
    let refreshed = readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("refresh index");
    let mut incompatible = refreshed.manifest;
    incompatible.tokenizer_version = "future-tokenizer-v99".to_owned();
    incompatible.manifest_digest = manifest_digest(&incompatible);
    update_index(
        &fixture.index_path,
        "UPDATE manifest SET tokenizer_version = ?1, manifest_digest = ?2",
        params![
            incompatible.tokenizer_version,
            incompatible.manifest_digest.to_string()
        ],
    );
    let result = readonly
        .search(fixture.query_options("incompatible_manifest_marker"))
        .expect("incompatible diagnostic");
    assert_eq!(result.status, SearchDerivedStatus::RefreshRequired);
    assert_eq!(result.reason_code.as_deref(), Some("incompatible_manifest"));
    assert!(result.items.is_empty());
    assert!(
        readonly
            .refresh_search_index(fixture.refresh_options())
            .expect_err("incompatible existing generation is immutable")
            .to_string()
            .contains("select a new index path")
    );
}

#[cfg(unix)]
#[test]
fn interrupted_refresh_preserves_the_last_complete_generation() {
    use std::os::unix::fs::PermissionsExt;

    let (fixture, mut engine) = Fixture::new("search-interrupted");
    let first = engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "first_generation_marker remains published",
            )
            .unwrap(),
        )
        .expect("first record");
    drop(engine);
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly first source");
    readonly
        .refresh_search_index(fixture.refresh_options())
        .expect("initial refresh");
    drop(readonly);
    let published_before = fs::read(&fixture.index_path).expect("published generation");

    let mut writer = Engine::open(&fixture.source_path).expect("writer");
    writer
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                first.commit_id,
                "second_generation_marker requires refresh",
            )
            .unwrap(),
        )
        .expect("advance source");
    drop(writer);

    let replacement_path = fixture._tempdir.path().join("replacement-search.sqlite");
    let parent = replacement_path.parent().unwrap();
    let original_permissions = fs::metadata(parent).unwrap().permissions();
    let mut read_only_permissions = original_permissions.clone();
    read_only_permissions.set_mode(0o500);
    fs::set_permissions(parent, read_only_permissions).expect("make destination unwritable");
    let attempt = Engine::open_readonly(&fixture.source_path).and_then(|engine| {
        engine.refresh_search_index(SearchRefreshOptions::new(
            fixture.identity(),
            &replacement_path,
        ))
    });
    fs::set_permissions(parent, original_permissions).expect("restore destination permissions");
    assert!(
        attempt.is_err(),
        "refresh must fail when temp creation is denied"
    );
    assert_eq!(
        fs::read(&fixture.index_path).expect("preserved generation"),
        published_before,
        "failed refresh must leave the previous publication byte-for-byte intact"
    );

    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly updated source");
    let stale = readonly
        .search(fixture.query_options("first_generation_marker"))
        .expect("old generation remains diagnosable");
    assert_eq!(stale.reason_code.as_deref(), Some("stale_index"));
    readonly
        .refresh_search_index(SearchRefreshOptions::new(
            fixture.identity(),
            &replacement_path,
        ))
        .expect("refresh after write access restored");
    let ready = readonly
        .search(
            SearchQueryOptions::new(
                fixture.identity(),
                &replacement_path,
                "second_generation_marker",
            )
            .unwrap(),
        )
        .expect("query replacement generation");
    assert_eq!(ready.status, SearchDerivedStatus::Ready);
}

#[test]
fn concurrent_refreshes_publish_one_complete_index_without_touching_source() {
    let (fixture, mut engine) = Fixture::new("search-concurrent");
    engine
        .create_record(
            RecordCreateOptions::finding(
                fixture.workspace.initial_branch_id,
                fixture.workspace.genesis_commit_id,
                "concurrent_refresh_marker atomic publication",
            )
            .unwrap(),
        )
        .expect("record");
    drop(engine);
    let source_before = fs::read(&fixture.source_path).expect("source before");
    let barrier = Arc::new(Barrier::new(3));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let barrier = Arc::clone(&barrier);
        let source_path = fixture.source_path.clone();
        let options = fixture.refresh_options();
        threads.push(std::thread::spawn(move || {
            let readonly = Engine::open_readonly(source_path).expect("thread readonly source");
            barrier.wait();
            readonly.refresh_search_index(options)
        }));
    }
    barrier.wait();
    for thread in threads {
        thread
            .join()
            .expect("refresh thread")
            .expect("concurrent refresh");
    }
    let readonly = Engine::open_readonly(&fixture.source_path).expect("readonly final source");
    let result = readonly
        .search(fixture.query_options("concurrent_refresh_marker"))
        .expect("query concurrently published index");
    assert_eq!(result.status, SearchDerivedStatus::Ready);
    assert_eq!(result.items.len(), 1);
    assert_eq!(fs::read(&fixture.source_path).unwrap(), source_before);
    let temporary_files = fs::read_dir(fixture._tempdir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains("workvcs-search")
        })
        .count();
    assert_eq!(temporary_files, 0, "refresh must clean temporary files");
}
