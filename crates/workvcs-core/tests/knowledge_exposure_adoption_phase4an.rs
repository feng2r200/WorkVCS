use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, CommitId, Engine, KnowledgeCreateCommit, KnowledgeCreateOptions,
    KnowledgeExposureAdoptOptions, KnowledgeExposureCreateLocalOptions,
    KnowledgeExposureRefreshSourceStatusOptions, KnowledgeSpaceCreateOptions,
    KnowledgeTransitionOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4an-knowledge-exposure-adoption-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn object(entries: Vec<(&str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
    .expect("canonical object")
}

fn create_source_knowledge(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: CommitId,
) -> KnowledgeCreateCommit {
    engine
        .create_knowledge(
            KnowledgeCreateOptions::new(workspace.initial_branch_id, head, "Reusable source")
                .expect("knowledge options")
                .with_scope(object(vec![(
                    "domain",
                    CanonicalValue::String("research".to_owned()),
                )]))
                .expect("knowledge scope")
                .with_provenance(object(vec![(
                    "source",
                    CanonicalValue::String("curated".to_owned()),
                )]))
                .expect("knowledge provenance"),
        )
        .expect("create source knowledge")
}

fn object_entry<'a>(value: &'a CanonicalValue, key: &str) -> &'a CanonicalValue {
    let CanonicalValue::Object(entries) = value else {
        panic!("expected canonical object");
    };
    entries
        .iter()
        .find_map(|(entry_key, entry_value)| (entry_key == key).then_some(entry_value))
        .unwrap_or_else(|| panic!("missing key {key}"))
}

#[test]
fn adopts_current_exposure_as_workspace_knowledge_and_links_provenance() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let source_knowledge =
        create_source_knowledge(&mut engine, &workspace, workspace.genesis_commit_id);
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                source_knowledge.knowledge_entity_id,
                source_knowledge.knowledge_entity_version_id,
            )
            .expect("exposure options"),
        )
        .expect("create exposure")
        .exposure;

    let adoption = engine
        .adopt_knowledge_exposure(
            KnowledgeExposureAdoptOptions::new(
                workspace.initial_branch_id,
                source_knowledge.commit_id,
                exposure.exposure_id,
                "adopt current exposure",
            )
            .expect("adoption options"),
        )
        .expect("adopt exposure");

    assert_eq!(
        adoption.candidate.exposure.exposure_id,
        exposure.exposure_id
    );
    assert_eq!(
        adoption.knowledge.previous_head_commit_id,
        source_knowledge.commit_id
    );
    assert_eq!(adoption.knowledge.state.statement, "Reusable source");
    assert_eq!(adoption.knowledge.state.scope, source_knowledge.state.scope);
    assert_eq!(
        object_entry(&adoption.knowledge.state.provenance, "source_provenance"),
        &source_knowledge.state.provenance
    );
    assert_eq!(
        object_entry(&adoption.knowledge.state.provenance, "kind"),
        &CanonicalValue::String("knowledge_exposure_adoption_v1".to_owned())
    );
    assert_eq!(
        object_entry(
            object_entry(&adoption.knowledge.state.provenance, "adoption"),
            "adopted_from_exposure_id"
        ),
        &CanonicalValue::String(exposure.exposure_id.to_string())
    );

    assert_eq!(
        adoption.relation.previous_head_commit_id,
        adoption.knowledge.commit_id
    );
    assert_eq!(
        adoption.relation.knowledge_entity_id,
        adoption.knowledge.knowledge_entity_id
    );
    assert_eq!(adoption.relation.exposure_id, exposure.exposure_id);
    assert_eq!(adoption.relation.relation_type, "derived_from");
    let branch = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(branch.head_commit_id, adoption.relation.commit_id);
    let adopted = engine
        .knowledge_at(
            adoption.relation.commit_id,
            adoption.knowledge.knowledge_entity_id,
        )
        .expect("adopted knowledge at final head");
    assert_eq!(
        adopted.knowledge_entity_version_id,
        adoption.knowledge.knowledge_entity_version_id
    );
    let replayed = engine
        .state_at(adoption.relation.commit_id)
        .expect("replay adoption");
    assert!(
        replayed
            .state
            .relations()
            .iter()
            .any(|(relation_id, relation_version_id)| {
                *relation_id == adoption.relation.relation_id
                    && *relation_version_id == adoption.relation.relation_version_id
            })
    );
    assert_eq!(replayed.state_digest, adoption.relation.work_state_digest);
}

#[test]
fn rejects_stale_exposure_adoption_until_policy_is_defined() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let source_knowledge =
        create_source_knowledge(&mut engine, &workspace, workspace.genesis_commit_id);
    let knowledge_space = engine
        .create_knowledge_space(KnowledgeSpaceCreateOptions::new("Research").expect("space name"))
        .expect("create knowledge space")
        .knowledge_space;
    let exposure = engine
        .create_local_knowledge_exposure(
            KnowledgeExposureCreateLocalOptions::new(
                knowledge_space.knowledge_space_id,
                workspace.workspace_id,
                source_knowledge.knowledge_entity_id,
                source_knowledge.knowledge_entity_version_id,
            )
            .expect("exposure options"),
        )
        .expect("create exposure")
        .exposure;
    let invalidated = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::invalidate(
                workspace.initial_branch_id,
                source_knowledge.commit_id,
                source_knowledge.knowledge_entity_id,
                source_knowledge.knowledge_entity_version_id,
                "source drift",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate source knowledge");
    let refreshed = engine
        .refresh_knowledge_exposure_source_status(KnowledgeExposureRefreshSourceStatusOptions::new(
            exposure.exposure_id,
        ))
        .expect("refresh source status");
    assert_eq!(
        refreshed.exposure.source_status.source_status.as_str(),
        "stale"
    );

    let error = engine
        .adopt_knowledge_exposure(
            KnowledgeExposureAdoptOptions::new(
                workspace.initial_branch_id,
                invalidated.commit_id,
                exposure.exposure_id,
                "adopt stale exposure",
            )
            .expect("adoption options"),
        )
        .expect_err("stale exposure adoption rejected");

    assert!(error.to_string().contains("not current"));
}
