use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ContextItemCategory, ContextPacketOptions, ContextPriority, ContextProfile, Engine, ErrorCode,
    KnowledgeCreateOptions, RecordCreateOptions, RecordTransitionOptions, SessionId,
    SessionStartOptions, StoreInitOptions, TaskCreateOptions, TaskSchedulingRelationCreateOptions,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase4kx-context-profile-budget-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn contains_category(packet: &workvcs_core::ContextPacket, category: ContextItemCategory) -> bool {
    packet.items.iter().any(|item| item.category == category)
}

#[test]
fn context_packet_profiles_filter_categories_deterministically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Implement context packet")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;

    let decision = engine
        .create_record(
            RecordCreateOptions::decision(branch_id, head, "Use deterministic packet profiles")
                .expect("decision options"),
        )
        .expect("create decision");
    head = decision.commit_id;

    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(branch_id, head, "Agents need bounded context")
                .expect("assumption options"),
        )
        .expect("create assumption");
    head = assumption.commit_id;

    let attempt = engine
        .create_record(
            RecordCreateOptions::attempt(branch_id, head, "Prototype string truncation")
                .expect("attempt options"),
        )
        .expect("create attempt");
    head = attempt.commit_id;
    let failed_attempt = engine
        .transition_record(
            RecordTransitionOptions::complete_attempt_failed(
                branch_id,
                head,
                attempt.record_entity_id,
                attempt.record_entity_version_id,
                "Item omission preserves packet structure",
            )
            .expect("failed attempt options"),
        )
        .expect("fail attempt");
    head = failed_attempt.commit_id;

    let finding = engine
        .create_record(
            RecordCreateOptions::finding(branch_id, head, "Budget summaries are inspectable")
                .expect("finding options"),
        )
        .expect("create finding");
    head = finding.commit_id;

    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Context profile names are V1 vocabulary")
                .expect("knowledge options"),
        )
        .expect("create knowledge");
    head = knowledge.commit_id;

    let handoff = engine
        .create_record(
            RecordCreateOptions::handoff(branch_id, head, "Continue with verify wrapper next")
                .expect("handoff options"),
        )
        .expect("create handoff");
    head = handoff.commit_id;

    let risk = engine
        .create_record(
            RecordCreateOptions::risk(branch_id, head, "Do not claim full context resolver yet")
                .expect("risk options"),
        )
        .expect("create risk");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let brief = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief),
        )
        .expect("brief context packet");
    let normal = engine
        .context_packet(ContextPacketOptions::new(session.session_id))
        .expect("normal context packet");
    let full = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Full),
        )
        .expect("full context packet");

    assert_eq!(normal.profile, ContextProfile::Normal);
    assert_eq!(brief.budget_items, None);
    assert_eq!(brief.omission_summary.total, 0);
    assert_eq!(normal.omission_summary.total, 0);
    assert_eq!(full.omission_summary.total, 0);
    assert_eq!(full.envelope.head_commit_id, risk.commit_id);

    assert!(contains_category(
        &brief,
        ContextItemCategory::SessionAnchor
    ));
    assert!(contains_category(
        &brief,
        ContextItemCategory::BranchOverview
    ));
    assert!(contains_category(&brief, ContextItemCategory::CurrentTask));
    assert!(contains_category(
        &brief,
        ContextItemCategory::TaskReadiness
    ));
    assert!(contains_category(
        &brief,
        ContextItemCategory::ActiveDecision
    ));
    assert!(contains_category(
        &brief,
        ContextItemCategory::ActiveAssumption
    ));
    assert!(contains_category(
        &brief,
        ContextItemCategory::FailedAttempt
    ));
    assert!(!contains_category(&brief, ContextItemCategory::Finding));
    assert!(!contains_category(
        &brief,
        ContextItemCategory::ScopedKnowledge
    ));
    assert!(!contains_category(
        &brief,
        ContextItemCategory::RelevantHandoff
    ));
    assert!(!contains_category(
        &brief,
        ContextItemCategory::OlderProvenance
    ));

    assert!(contains_category(&normal, ContextItemCategory::Finding));
    assert!(contains_category(
        &normal,
        ContextItemCategory::ScopedKnowledge
    ));
    assert!(contains_category(
        &normal,
        ContextItemCategory::RelevantHandoff
    ));
    assert!(!contains_category(
        &normal,
        ContextItemCategory::OlderProvenance
    ));

    assert!(contains_category(
        &full,
        ContextItemCategory::OlderProvenance
    ));
    assert!(normal.items.len() > brief.items.len());
    assert!(full.items.len() > normal.items.len());
}

#[test]
fn context_packet_budget_omits_whole_low_priority_items() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Budgeted context task").expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(branch_id, head, "Low-priority finding")
                .expect("finding options"),
        )
        .expect("create finding");
    head = finding.commit_id;
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Scoped knowledge can be omitted")
                .expect("knowledge options"),
        )
        .expect("create knowledge");
    head = knowledge.commit_id;
    let risk = engine
        .create_record(
            RecordCreateOptions::risk(branch_id, head, "Full-only provenance")
                .expect("risk options"),
        )
        .expect("create risk");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Full)
                .with_budget_items(3)
                .expect("budgeted context options"),
        )
        .expect("budgeted context packet");

    assert_eq!(packet.items.len(), 3);
    assert_eq!(packet.available_items, 8);
    assert_eq!(packet.omission_summary.total, 5);
    assert_eq!(packet.envelope.head_commit_id, risk.commit_id);
    assert!(
        packet
            .items
            .iter()
            .all(|item| item.priority == ContextPriority::P0)
    );
    assert!(
        packet
            .omission_summary
            .by_priority
            .iter()
            .any(|bucket| bucket.priority == ContextPriority::P2 && bucket.omitted == 1)
    );
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::ScopedKnowledge
                    && bucket.omitted == 1
            )
    );
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::OlderProvenance
                    && bucket.omitted == 1
            )
    );
}

#[test]
fn context_packet_budget_preserves_runnable_order_within_current_tasks() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let dependent = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Blocked dependent task")
                .expect("dependent task options"),
        )
        .expect("create dependent task");
    head = dependent.commit_id;
    let prerequisite = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Ready prerequisite task")
                .expect("prerequisite task options"),
        )
        .expect("create prerequisite task");
    head = prerequisite.commit_id;
    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                branch_id,
                head,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("dependency options"),
        )
        .expect("create dependency");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Brief)
                .with_budget_items(3)
                .expect("budgeted context options"),
        )
        .expect("budgeted context packet");

    assert_eq!(packet.envelope.head_commit_id, relation.commit_id);
    assert_eq!(packet.items.len(), 3);
    assert_eq!(packet.items[2].category, ContextItemCategory::CurrentTask);
    assert_eq!(
        packet.items[2].subject.as_ref_string(),
        format!("task:{}", prerequisite.task_entity_id)
    );
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::CurrentTask && bucket.omitted == 1
            )
    );
}

#[test]
fn context_packet_rejects_zero_item_budget() {
    let (_tempdir, path) = store_path();
    let (_engine, _workspace) = create_workspace(&path);

    let error = ContextPacketOptions::new(SessionId::new_v7())
        .with_budget_items(0)
        .expect_err("zero context item budget should fail");

    assert_eq!(error.code(), ErrorCode::QueryInvalid);
}
