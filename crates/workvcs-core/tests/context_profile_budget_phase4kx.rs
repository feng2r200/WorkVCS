use rusqlite::params;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions, CanonicalValue,
    ContextItemCategory, ContextPacket, ContextPacketListOptions, ContextPacketOptions,
    ContextPriority, ContextProfile, Engine, ErrorCode, GoalCreateOptions, GoalTransitionOptions,
    KnowledgeCreateOptions, PlanCreateOptions, PrimaryContainmentCreateOptions,
    RecordCreateOptions, RecordRelationCreateOptions, RecordTransitionOptions,
    ResourceCreateOptions, ResourceObservationCreateOptions, RunnableTasksOptions,
    SessionFocusOptions, SessionId, SessionStartOptions, StoreInitOptions, TaskCreateOptions,
    TaskSchedulingRelationCreateOptions, VerificationCreateOptions,
    VerificationRequirementCreateOptions, VerificationResourceBasis, VerificationResult,
    VerificationTarget, WorkVcsError, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes,
    content_object_digest,
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

fn object(entries: Vec<(&str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
    .expect("canonical object")
}

fn string(value: &str) -> CanonicalValue {
    CanonicalValue::String(value.to_owned())
}

fn array(values: Vec<CanonicalValue>) -> CanonicalValue {
    CanonicalValue::Array(values)
}

fn resource_basis(
    engine: &mut Engine,
    path: &str,
) -> (
    VerificationResourceBasis,
    workvcs_core::ResourceObservationId,
) {
    let resource = engine
        .create_resource(ResourceCreateOptions::new("local-file").expect("resource options"))
        .expect("create resource");
    let fingerprint = content_object_digest(format!("resource basis for {path}").as_bytes());
    let observation = engine
        .record_resource_observation(
            ResourceObservationCreateOptions::new(
                resource.resource_id,
                "local-file",
                1,
                fingerprint,
                object(vec![("path", string(path))]),
            )
            .expect("resource observation options"),
        )
        .expect("record resource observation");
    let basis = VerificationResourceBasis::new(
        resource.resource_id,
        "local-file",
        1,
        "path",
        1,
        object(vec![("path", string(path))]),
        fingerprint,
    )
    .expect("resource basis")
    .with_baseline_observation_id(observation.observation_id)
    .expect("baseline observation basis");
    (basis, observation.observation_id)
}

fn resource_basis_without_baseline(engine: &mut Engine, path: &str) -> VerificationResourceBasis {
    let resource = engine
        .create_resource(ResourceCreateOptions::new("local-file").expect("resource options"))
        .expect("create resource");
    let fingerprint =
        content_object_digest(format!("unobserved resource basis for {path}").as_bytes());
    VerificationResourceBasis::new(
        resource.resource_id,
        "local-file",
        1,
        "path",
        1,
        object(vec![("path", string(path))]),
        fingerprint,
    )
    .expect("resource basis without baseline")
}

fn scoped_knowledge_summaries(packet: &ContextPacket) -> Vec<&str> {
    packet
        .items
        .iter()
        .filter_map(|item| {
            (item.category == ContextItemCategory::ScopedKnowledge).then_some(item.summary.as_str())
        })
        .collect()
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
    assert!(contains_category(
        &brief,
        ContextItemCategory::TransitionRationale
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
fn context_packet_includes_recent_transition_rationales() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let goal = engine
        .create_goal(
            GoalCreateOptions::new(branch_id, head, "Close transition rationale context")
                .expect("goal options"),
        )
        .expect("create goal");
    head = goal.commit_id;
    let achieved = engine
        .transition_goal(
            GoalTransitionOptions::achieve(
                branch_id,
                head,
                goal.goal_entity_id,
                goal.goal_entity_version_id,
                "transition rationale helps continuation",
            )
            .expect("goal transition options"),
        )
        .expect("achieve goal");
    head = achieved.commit_id;
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(branch_id, head, "Later empty-rationale write")
                .expect("finding options"),
        )
        .expect("create finding");

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
        .expect("context packet");

    assert_eq!(packet.envelope.head_commit_id, finding.commit_id);
    assert_eq!(packet.available_items, 3);
    assert_eq!(packet.items.len(), 3);
    let rationale_item = packet
        .items
        .iter()
        .find(|item| item.category == ContextItemCategory::TransitionRationale)
        .expect("transition rationale item");
    assert_eq!(rationale_item.priority, ContextPriority::P3);
    assert_eq!(
        rationale_item.subject.as_ref_string(),
        format!("changeset:{}@{}", achieved.changeset_id, achieved.commit_id)
    );
    assert!(
        rationale_item
            .summary
            .contains("operation=entity.transition")
    );
    assert!(
        rationale_item
            .summary
            .contains(&format!("changeset={}", achieved.changeset_id))
    );
    assert!(
        rationale_item
            .summary
            .contains("rationale_json={\"reason\":\"transition rationale helps continuation\"}")
    );
    assert!(
        !rationale_item
            .summary
            .contains("Later empty-rationale write")
    );

    let tight_packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Brief)
                .with_budget_items(2)
                .expect("tight budgeted context options"),
        )
        .expect("tight context packet");
    assert_eq!(tight_packet.items.len(), 2);
    assert!(
        !tight_packet
            .items
            .iter()
            .any(|item| item.category == ContextItemCategory::TransitionRationale)
    );
    assert!(
        tight_packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::TransitionRationale
                    && bucket.omitted == 1
            )
    );

    let saved = engine
        .save_context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Brief)
                .with_budget_items(3)
                .expect("saved packet options"),
        )
        .expect("save context packet");
    let packet_json = String::from_utf8(canonical_bytes(&saved.snapshot.packet_json).unwrap())
        .expect("packet JSON is UTF-8");
    assert_eq!(saved.snapshot.item_count, 3);
    assert!(packet_json.contains("transition_rationale"));
    assert!(packet_json.contains("transition rationale helps continuation"));
}

#[test]
fn context_packet_filters_path_scoped_knowledge_by_explicit_scope() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Filter scoped knowledge")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;

    let global = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Global knowledge remains visible")
                .expect("global knowledge options"),
        )
        .expect("create global knowledge");
    head = global.commit_id;

    let exact = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Exact path knowledge remains visible")
                .expect("exact knowledge options")
                .with_scope(object(vec![(
                    "path",
                    string("crates/workvcs-core/src/runtime/context.rs"),
                )]))
                .expect("exact knowledge scope"),
        )
        .expect("create exact knowledge");
    head = exact.commit_id;

    let prefix = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Prefix path knowledge remains visible")
                .expect("prefix knowledge options")
                .with_scope(object(vec![(
                    "path_prefix",
                    string("crates/workvcs-core/src/runtime"),
                )]))
                .expect("prefix knowledge scope"),
        )
        .expect("create prefix knowledge");
    head = prefix.commit_id;

    let payload = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                branch_id,
                head,
                "Resource payload knowledge remains visible",
            )
            .expect("payload knowledge options")
            .with_scope(object(vec![
                ("scope_kind", string("resource")),
                (
                    "scope_payload",
                    object(vec![(
                        "path",
                        string("crates/workvcs-core/src/runtime/context.rs"),
                    )]),
                ),
            ]))
            .expect("payload knowledge scope"),
        )
        .expect("create payload knowledge");
    head = payload.commit_id;

    let non_path = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                branch_id,
                head,
                "Non-path scoped knowledge remains visible",
            )
            .expect("non-path knowledge options")
            .with_scope(object(vec![("kind", string("workspace"))]))
            .expect("non-path knowledge scope"),
        )
        .expect("create non-path knowledge");
    head = non_path.commit_id;

    let other = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Other path knowledge is filtered")
                .expect("other knowledge options")
                .with_scope(object(vec![(
                    "paths",
                    array(vec![string("crates/workvcs-cli/src/main.rs")]),
                )]))
                .expect("other knowledge scope"),
        )
        .expect("create other knowledge");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let unscoped = engine
        .context_packet(ContextPacketOptions::new(session.session_id))
        .expect("unscoped context packet");
    assert_eq!(scoped_knowledge_summaries(&unscoped).len(), 6);

    let scope = object(vec![(
        "path",
        string("crates/workvcs-core/src/runtime/context.rs"),
    )]);
    let scoped = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_scope(scope.clone())
                .expect("scoped context options"),
        )
        .expect("scoped context packet");

    assert_eq!(scoped.envelope.head_commit_id, other.commit_id);
    assert_eq!(scoped.scope.as_ref(), Some(&scope));
    let summaries = scoped_knowledge_summaries(&scoped);
    assert_eq!(summaries.len(), 5);
    assert!(
        summaries
            .iter()
            .any(|summary| summary.contains("Global knowledge remains visible"))
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.contains("Exact path knowledge remains visible"))
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.contains("Prefix path knowledge remains visible"))
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.contains("Resource payload knowledge remains visible"))
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.contains("Non-path scoped knowledge remains visible"))
    );
    assert!(
        !summaries
            .iter()
            .any(|summary| summary.contains("Other path knowledge is filtered"))
    );
    assert_eq!(unscoped.available_items, scoped.available_items + 1);

    let budgeted = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_scope(scope)
                .expect("budgeted scoped context options")
                .with_budget_items(4)
                .expect("budgeted scoped context options"),
        )
        .expect("budgeted scoped context packet");
    assert_eq!(budgeted.available_items, scoped.available_items);
    assert_eq!(budgeted.items.len(), 4);
    assert_eq!(budgeted.omission_summary.total, scoped.available_items - 4);
    assert!(
        budgeted
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::ScopedKnowledge
                    && bucket.omitted == 5
            )
    );
}

#[test]
fn context_packet_path_scope_matching_normalizes_lexical_variants() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Normalize scoped path selectors")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;

    let exact = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Normalized exact path remains visible")
                .expect("exact knowledge options")
                .with_scope(object(vec![(
                    "path",
                    string("/repo/work/../work/src/./lib.rs"),
                )]))
                .expect("exact knowledge scope"),
        )
        .expect("create exact knowledge");
    head = exact.commit_id;

    let prefix = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Normalized prefix remains visible")
                .expect("prefix knowledge options")
                .with_scope(object(vec![("path_prefix", string("/repo/work/src//"))]))
                .expect("prefix knowledge scope"),
        )
        .expect("create prefix knowledge");
    head = prefix.commit_id;

    let unrelated = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Lexically adjacent path is filtered")
                .expect("unrelated knowledge options")
                .with_scope(object(vec![(
                    "path",
                    string("/repo/work/src-other/lib.rs"),
                )]))
                .expect("unrelated knowledge scope"),
        )
        .expect("create unrelated knowledge");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let scope = object(vec![("path", string("/repo/work/src/./lib.rs"))]);
    let scoped = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_scope(scope.clone())
                .expect("scoped context options"),
        )
        .expect("scoped context packet");

    assert_eq!(scoped.envelope.head_commit_id, unrelated.commit_id);
    assert_eq!(scoped.scope.as_ref(), Some(&scope));
    let summaries = scoped_knowledge_summaries(&scoped);
    assert_eq!(summaries.len(), 2);
    assert!(
        summaries
            .iter()
            .any(|summary| summary.contains("Normalized exact path remains visible"))
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.contains("Normalized prefix remains visible"))
    );
    assert!(
        !summaries
            .iter()
            .any(|summary| summary.contains("Lexically adjacent path is filtered"))
    );
}

#[test]
fn context_packet_snapshots_persist_scope_json_and_stable_packet_digest() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Persist resolved context packet")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;

    let matching = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Persisted packet keeps matching path")
                .expect("matching knowledge options")
                .with_scope(object(vec![(
                    "path",
                    string("crates/workvcs-core/src/runtime/context.rs"),
                )]))
                .expect("matching knowledge scope"),
        )
        .expect("create matching knowledge");
    head = matching.commit_id;

    let unrelated = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(branch_id, head, "Persisted packet filters other path")
                .expect("unrelated knowledge options")
                .with_scope(object(vec![(
                    "path",
                    string("crates/workvcs-cli/src/main.rs"),
                )]))
                .expect("unrelated knowledge scope"),
        )
        .expect("create unrelated knowledge");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let scope = object(vec![(
        "path",
        string("crates/workvcs-core/src/runtime/context.rs"),
    )]);
    let options = ContextPacketOptions::new(session.session_id)
        .with_profile(ContextProfile::Normal)
        .with_budget_items(8)
        .expect("budgeted context options")
        .with_scope(scope.clone())
        .expect("scoped context options");

    let saved = engine
        .save_context_packet(options.clone())
        .expect("save context packet");
    assert_eq!(saved.snapshot.session_id, session.session_id);
    assert_eq!(saved.snapshot.workspace_id, workspace.workspace_id);
    assert_eq!(saved.snapshot.branch_id, branch_id);
    assert_eq!(saved.snapshot.head_commit_id, unrelated.commit_id);
    assert_eq!(
        saved.snapshot.state_digest,
        saved.packet.envelope.state_digest
    );
    assert_eq!(saved.snapshot.profile, ContextProfile::Normal);
    assert_eq!(
        saved.snapshot.budget_items.map(|value| value.get()),
        Some(8)
    );
    assert_eq!(saved.snapshot.scope.as_ref(), Some(&scope));
    assert_eq!(saved.snapshot.available_items, saved.packet.available_items);
    assert_eq!(saved.snapshot.item_count, saved.packet.items.len());
    assert_eq!(
        saved.snapshot.omitted_items,
        saved.packet.omission_summary.total
    );

    let packet_json =
        String::from_utf8(canonical_bytes(&saved.snapshot.packet_json).expect("json"))
            .expect("packet json is utf8");
    assert!(packet_json.contains("\"context_packet_format\":\"workvcs-context-packet-v1\""));
    assert!(packet_json.contains("\"context_packet_format_version\":1"));
    assert!(
        packet_json.contains("\"scope\":{\"path\":\"crates/workvcs-core/src/runtime/context.rs\"}")
    );
    assert!(packet_json.contains("Persisted packet keeps matching path"));
    assert!(!packet_json.contains("Persisted packet filters other path"));

    let loaded = engine
        .context_packet_snapshot(saved.snapshot.context_packet_id)
        .expect("load context packet snapshot");
    assert_eq!(loaded, saved.snapshot);

    {
        let connection = rusqlite::Connection::open(&path).expect("open raw sqlite");
        let context_packet_id_bytes = saved.snapshot.context_packet_id.raw_bytes();
        connection
            .execute(
                "UPDATE context_packet_snapshot
                 SET available_items = available_items + 1
                 WHERE context_packet_id = ?1",
                params![&context_packet_id_bytes[..]],
            )
            .expect("corrupt context packet snapshot count");
    }
    let corrupt_snapshot = engine.context_packet_snapshot(saved.snapshot.context_packet_id);
    assert!(matches!(
        corrupt_snapshot,
        Err(WorkVcsError::StorageFailure(message))
            if message.contains("available_items does not match packet_json")
    ));

    {
        let connection = rusqlite::Connection::open(&path).expect("open raw sqlite");
        let context_packet_id_bytes = saved.snapshot.context_packet_id.raw_bytes();
        connection
            .execute(
                "UPDATE context_packet_snapshot
                 SET available_items = available_items - 1
                 WHERE context_packet_id = ?1",
                params![&context_packet_id_bytes[..]],
            )
            .expect("repair context packet snapshot count");
    }

    let saved_again = engine
        .save_context_packet(options)
        .expect("save same context packet again");
    assert_ne!(
        saved_again.snapshot.context_packet_id,
        saved.snapshot.context_packet_id
    );
    assert_eq!(
        saved_again.snapshot.packet_digest,
        saved.snapshot.packet_digest
    );
    assert_eq!(saved_again.snapshot.packet_json, saved.snapshot.packet_json);

    let listed = engine
        .context_packet_snapshots(
            ContextPacketListOptions::for_session(session.session_id)
                .with_limit(10)
                .expect("list limit"),
        )
        .expect("list context packet snapshots");
    assert_eq!(listed.session_id, session.session_id);
    assert_eq!(listed.snapshots.len(), 2);
    assert_eq!(
        listed.snapshots[0].context_packet_id,
        saved_again.snapshot.context_packet_id
    );
    assert_eq!(
        listed.snapshots[0].packet_digest,
        saved.snapshot.packet_digest
    );
    assert_eq!(
        listed.snapshots[1].context_packet_id,
        saved.snapshot.context_packet_id
    );

    assert!(
        ContextPacketListOptions::for_session(session.session_id)
            .with_limit(0)
            .is_err()
    );
}

#[test]
fn context_packet_summarizes_attempt_execution_detail() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let running = engine
        .create_record(
            RecordCreateOptions::attempt(branch_id, head, "Retry parser with bounded input")
                .expect("running attempt options"),
        )
        .expect("create running attempt");
    head = running.commit_id;

    let succeeded_started = engine
        .create_record(
            RecordCreateOptions::attempt(
                branch_id,
                head,
                "Prove normal profile attempt visibility",
            )
            .expect("succeeded attempt options"),
        )
        .expect("create succeeded attempt");
    head = succeeded_started.commit_id;
    let succeeded = engine
        .transition_record(
            RecordTransitionOptions::complete_attempt_succeeded(
                branch_id,
                head,
                succeeded_started.record_entity_id,
                succeeded_started.record_entity_version_id,
                "Normal profile can keep completed Attempt detail",
            )
            .expect("succeeded transition options"),
        )
        .expect("succeed attempt");
    head = succeeded.commit_id;

    let failed_started = engine
        .create_record(
            RecordCreateOptions::attempt(branch_id, head, "Capture failed attempt for packet")
                .expect("failed attempt options"),
        )
        .expect("create failed attempt");
    head = failed_started.commit_id;
    let failed = engine
        .transition_record(
            RecordTransitionOptions::complete_attempt_failed(
                branch_id,
                head,
                failed_started.record_entity_id,
                failed_started.record_entity_version_id,
                "Failed attempt should be brief critical context",
            )
            .expect("failed transition options"),
        )
        .expect("fail attempt");
    head = failed.commit_id;

    let inconclusive_started = engine
        .create_record(
            RecordCreateOptions::attempt(branch_id, head, "Check outcome when source is partial")
                .expect("inconclusive attempt options"),
        )
        .expect("create inconclusive attempt");
    head = inconclusive_started.commit_id;
    let inconclusive = engine
        .transition_record(
            RecordTransitionOptions::complete_attempt_inconclusive(
                branch_id,
                head,
                inconclusive_started.record_entity_id,
                inconclusive_started.record_entity_version_id,
                "Source evidence did not settle the result",
            )
            .expect("inconclusive transition options"),
        )
        .expect("mark attempt inconclusive");
    head = inconclusive.commit_id;

    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::related_to(
                branch_id,
                head,
                failed.record_entity_id,
                running.record_entity_id,
                "retry-context",
                "Failed attempt explains the still-running retry",
            )
            .expect("related attempt relation options"),
        )
        .expect("link failed attempt to retry");

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

    assert!(!contains_category(&brief, ContextItemCategory::Attempt));
    let failed_summary = brief
        .items
        .iter()
        .find(|item| item.category == ContextItemCategory::FailedAttempt)
        .expect("brief failed attempt item")
        .summary
        .as_str();
    assert!(failed_summary.contains("attempt detail status=failed terminal=true"));
    assert!(failed_summary.contains(&format!("record={}", failed.record_entity_id)));
    assert!(failed_summary.contains(&format!("version={}", failed.record_entity_version_id)));
    assert!(failed_summary.contains(&format!("state_digest={}", failed.record_state_digest)));
    assert!(failed_summary.contains("scope_json={}"));
    assert!(failed_summary.contains("record_relations_out=1"));
    assert!(failed_summary.contains("record_relations_in=0"));
    assert!(failed_summary.contains("record_relation_types=out:related_to=1"));
    assert!(failed_summary.ends_with("statement=Capture failed attempt for packet"));

    let attempt_summaries = normal
        .items
        .iter()
        .filter(|item| item.category == ContextItemCategory::Attempt)
        .map(|item| item.summary.as_str())
        .collect::<Vec<_>>();
    assert_eq!(attempt_summaries.len(), 3);
    assert!(attempt_summaries.iter().any(|summary| {
        summary.contains("attempt detail status=running terminal=false")
            && summary.contains(&format!("record={}", running.record_entity_id))
            && summary.contains("record_relations_in=1")
            && summary.contains("record_relation_types=in:related_to=1")
    }));
    assert!(attempt_summaries.iter().any(|summary| {
        summary.contains("attempt detail status=succeeded terminal=true")
            && summary.contains(&format!("record={}", succeeded.record_entity_id))
    }));
    assert!(attempt_summaries.iter().any(|summary| {
        summary.contains("attempt detail status=inconclusive terminal=true")
            && summary.contains(&format!("record={}", inconclusive.record_entity_id))
    }));

    assert!(normal.items.iter().any(|item| {
        item.category == ContextItemCategory::DirectCausalChain
            && item.subject.as_ref_string() == format!("relation:{}", relation.relation_id)
    }));
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
fn context_packet_includes_current_task_verification_obligations() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Verify packet obligations")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                branch_id,
                head,
                task.task_entity_id,
                task.task_entity_version_id,
                "ac-context",
                "Packet tells the agent what must be verified.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance criterion options"),
        )
        .expect("create acceptance criterion");
    head = criterion.commit_id;
    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                branch_id,
                head,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "vr-wrapper",
                "Run the verify wrapper with captured evidence.",
            )
            .expect("verification requirement options"),
        )
        .expect("create verification requirement");

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
                .with_budget_items(5)
                .expect("budgeted context options"),
        )
        .expect("budgeted context packet");

    assert_eq!(packet.envelope.head_commit_id, requirement.commit_id);
    assert_eq!(packet.available_items, 6);
    assert_eq!(packet.items.len(), 5);
    assert_eq!(packet.items[0].category, ContextItemCategory::SessionAnchor);
    assert_eq!(
        packet.items[1].category,
        ContextItemCategory::BranchOverview
    );
    assert_eq!(packet.items[2].category, ContextItemCategory::CurrentTask);
    assert_eq!(
        packet.items[3].category,
        ContextItemCategory::AcceptanceCriterion
    );
    assert_eq!(
        packet.items[3].subject.as_ref_string(),
        format!(
            "acceptance_criterion:{}",
            criterion.acceptance_criterion_entity_id
        )
    );
    assert!(packet.items[3].summary.contains("local_key=ac-context"));
    assert!(packet.items[3].summary.contains("classification=required"));
    assert!(packet.items[3].summary.contains("status=unverified"));
    assert!(packet.items[3].summary.contains("requirements=1"));
    assert!(
        packet.items[3]
            .summary
            .contains("Packet tells the agent what must be verified.")
    );
    assert_eq!(
        packet.items[4].category,
        ContextItemCategory::VerificationRequirement
    );
    assert_eq!(
        packet.items[4].subject.as_ref_string(),
        format!(
            "verification_requirement:{}",
            requirement.verification_requirement_entity_id
        )
    );
    assert!(packet.items[4].summary.contains(&format!(
        "criterion={}",
        criterion.acceptance_criterion_entity_id
    )));
    assert!(packet.items[4].summary.contains("local_key=vr-wrapper"));
    assert!(
        packet.items[4]
            .summary
            .contains("Run the verify wrapper with captured evidence.")
    );
    assert_eq!(packet.omission_summary.total, 1);
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::TaskReadiness
                    && bucket.omitted == 1
            )
    );
}

#[test]
fn context_packet_summarizes_resource_basis_recovery_for_current_task_requirements() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Recover resource-backed verification")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                branch_id,
                head,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-resource",
                "Packet must expose resource recovery without schema changes.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance criterion options"),
        )
        .expect("create acceptance criterion");
    head = criterion.commit_id;
    let backed_requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                branch_id,
                head,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-backed",
                "Refresh the local file basis.",
            )
            .expect("backed requirement options"),
        )
        .expect("create backed verification requirement");
    head = backed_requirement.commit_id;
    let plain_requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                branch_id,
                head,
                criterion.acceptance_criterion_entity_id,
                backed_requirement.acceptance_criterion_entity_version_id,
                "VR-plain",
                "Run the non-resource proof.",
            )
            .expect("plain requirement options"),
        )
        .expect("create plain verification requirement");
    head = plain_requirement.commit_id;

    let (basis, baseline_observation_id) = resource_basis(&mut engine, "docs/operator.md");
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                branch_id,
                head,
                VerificationTarget::VerificationRequirement(
                    backed_requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_resource_basis(vec![basis.clone()])
            .expect("verification resource basis"),
        )
        .expect("create resource-backed verification");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let options = ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief);
    let packet = engine
        .context_packet(options.clone())
        .expect("brief context packet");

    let backed_item = packet
        .items
        .iter()
        .find(|item| {
            item.category == ContextItemCategory::VerificationRequirement
                && item.summary.contains("local_key=VR-backed")
        })
        .expect("backed requirement context item");
    assert!(backed_item.summary.contains("resource_basis=1"));
    assert!(
        backed_item
            .summary
            .contains(&format!("resource_id={}", basis.resource_id))
    );
    assert!(backed_item.summary.contains("adapter=local-file@1"));
    assert!(backed_item.summary.contains("scope=path@1"));
    assert!(backed_item.summary.contains(&format!(
        "baseline_observation_id={baseline_observation_id}"
    )));
    assert!(backed_item.summary.contains(&format!(
        "refresh_hint=\"verification cache-refresh --verification {} --resource-content-from-basis\"",
        verification.verification_entity_id
    )));

    let plain_item = packet
        .items
        .iter()
        .find(|item| {
            item.category == ContextItemCategory::VerificationRequirement
                && item.summary.contains("local_key=VR-plain")
        })
        .expect("plain requirement context item");
    assert!(!plain_item.summary.contains("resource_basis="));
    assert!(!plain_item.summary.contains("refresh_hint="));

    let saved = engine
        .save_context_packet(options)
        .expect("save brief context packet");
    let packet_json =
        String::from_utf8(canonical_bytes(&saved.snapshot.packet_json).expect("json"))
            .expect("packet json is utf8");
    assert!(packet_json.contains("\"summary\""));
    assert!(!packet_json.contains("\"resource_basis\":"));
    assert!(!packet_json.contains("\"refresh_hint\":"));
}

#[test]
fn context_packet_prefers_refreshable_resource_basis_for_requirement_summary() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Prefer refreshable basis")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                branch_id,
                head,
                task.task_entity_id,
                task.task_entity_version_id,
                "AC-resource",
                "Packet should pick a basis-refreshable verification.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance criterion options"),
        )
        .expect("create acceptance criterion");
    head = criterion.commit_id;
    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                branch_id,
                head,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-refreshable",
                "Use the refreshable Resource basis.",
            )
            .expect("requirement options"),
        )
        .expect("create verification requirement");
    head = requirement.commit_id;

    let missing_baseline_basis =
        resource_basis_without_baseline(&mut engine, "docs/missing-baseline.md");
    let unrefreshable = engine
        .create_verification(
            VerificationCreateOptions::new(
                branch_id,
                head,
                VerificationTarget::VerificationRequirement(
                    requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("unrefreshable verification options")
            .with_resource_basis(vec![missing_baseline_basis])
            .expect("unrefreshable resource basis"),
        )
        .expect("create unrefreshable verification");
    head = unrefreshable.commit_id;

    let (refreshable_basis, baseline_observation_id) =
        resource_basis(&mut engine, "docs/refreshable.md");
    let refreshable = engine
        .create_verification(
            VerificationCreateOptions::new(
                branch_id,
                head,
                VerificationTarget::VerificationRequirement(
                    requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("refreshable verification options")
            .with_resource_basis(vec![refreshable_basis.clone()])
            .expect("refreshable resource basis"),
        )
        .expect("create refreshable verification");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief),
        )
        .expect("brief context packet");

    let item = packet
        .items
        .iter()
        .find(|item| {
            item.category == ContextItemCategory::VerificationRequirement
                && item.summary.contains("local_key=VR-refreshable")
        })
        .expect("requirement context item");
    assert!(item.summary.contains("resource_basis=2"));
    assert!(item.summary.contains(&format!(
        "verification_id={}",
        refreshable.verification_entity_id
    )));
    assert!(
        item.summary
            .contains(&format!("resource_id={}", refreshable_basis.resource_id))
    );
    assert!(item.summary.contains(&format!(
        "baseline_observation_id={baseline_observation_id}"
    )));
    assert!(item.summary.contains(&format!(
        "refresh_hint=\"verification cache-refresh --verification {} --resource-content-from-basis\"",
        refreshable.verification_entity_id
    )));
    assert!(
        !item
            .summary
            .contains("refresh_hint=unavailable_missing_baseline_observation")
    );
}

#[test]
fn context_packet_includes_goal_plan_path_for_current_tasks() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let goal = engine
        .create_goal(
            GoalCreateOptions::new(branch_id, head, "Ship the context resolver")
                .expect("goal options"),
        )
        .expect("create goal");
    head = goal.commit_id;
    let plan = engine
        .create_plan(
            PlanCreateOptions::new(
                branch_id,
                head,
                "Implement path packets",
                "Reuse current containment snapshots",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    head = plan.commit_id;
    let task = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Expose path to the agent")
                .expect("task options"),
        )
        .expect("create task");
    head = task.commit_id;
    let goal_plan = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch_id,
                head,
                goal.goal_entity_id,
                plan.plan_entity_id,
            )
            .expect("goal-plan containment options"),
        )
        .expect("create goal-plan containment");
    head = goal_plan.commit_id;
    let plan_task = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch_id,
                head,
                plan.plan_entity_id,
                task.task_entity_id,
            )
            .expect("plan-task containment options"),
        )
        .expect("create plan-task containment");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");

    let packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief),
        )
        .expect("brief context packet");

    assert_eq!(packet.envelope.head_commit_id, plan_task.commit_id);
    assert_eq!(packet.available_items, 5);
    assert_eq!(packet.items[0].category, ContextItemCategory::SessionAnchor);
    assert_eq!(
        packet.items[1].category,
        ContextItemCategory::BranchOverview
    );
    assert_eq!(packet.items[2].category, ContextItemCategory::CurrentTask);
    let path_item = packet
        .items
        .iter()
        .find(|item| item.category == ContextItemCategory::GoalPlanPath)
        .expect("goal plan path item");
    assert_eq!(path_item.priority, ContextPriority::P1);
    assert_eq!(
        path_item.subject.as_ref_string(),
        format!("goal_plan_path:{}", task.task_entity_id)
    );
    assert!(
        path_item
            .summary
            .contains(&format!("task={}", task.task_entity_id))
    );
    assert!(
        path_item
            .summary
            .contains(&format!("goal:{}", goal.goal_entity_id))
    );
    assert!(path_item.summary.contains("status=active"));
    assert!(path_item.summary.contains("Ship the context resolver"));
    assert!(
        path_item
            .summary
            .contains(&format!("plan:{}", plan.plan_entity_id))
    );
    assert!(path_item.summary.contains("Implement path packets"));
    assert!(
        path_item
            .summary
            .contains(&format!("relation={}", goal_plan.relation_id))
    );
    assert!(
        path_item
            .summary
            .contains(&format!("relation={}", plan_task.relation_id))
    );

    let tight_packet = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id)
                .with_profile(ContextProfile::Brief)
                .with_budget_items(3)
                .expect("budgeted context options"),
        )
        .expect("tight context packet");

    assert_eq!(tight_packet.items.len(), 3);
    assert!(
        !tight_packet
            .items
            .iter()
            .any(|item| item.category == ContextItemCategory::GoalPlanPath)
    );
    assert!(
        tight_packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::GoalPlanPath
                    && bucket.omitted == 1
            )
    );
}

#[test]
fn context_packet_explains_blocked_dependency_tasks() {
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
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief),
        )
        .expect("brief context packet");

    assert_eq!(packet.envelope.head_commit_id, relation.commit_id);
    let blocker = packet
        .items
        .iter()
        .find(|item| item.category == ContextItemCategory::BlockedDependency)
        .expect("blocked dependency item");
    assert_eq!(blocker.priority, ContextPriority::P2);
    assert_eq!(
        blocker.subject.as_ref_string(),
        format!(
            "blocked_dependency:{}:{}",
            dependent.task_entity_id, prerequisite.task_entity_id
        )
    );
    assert!(
        blocker
            .summary
            .contains(&format!("task={}", dependent.task_entity_id))
    );
    assert!(
        blocker
            .summary
            .contains(&format!("dependency={}", prerequisite.task_entity_id))
    );
    assert!(blocker.summary.contains("dependency_status=pending"));
    assert!(blocker.summary.contains("dependency_priority=0"));
    assert!(blocker.summary.contains("Ready prerequisite task"));
}

#[test]
fn context_packet_summarizes_resource_basis_recovery_for_blocked_dependency_requirements() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let dependent = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Blocked dependent needs prerequisite")
                .expect("dependent task options"),
        )
        .expect("create dependent task");
    head = dependent.commit_id;
    let prerequisite = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Prerequisite has resource-backed proof")
                .expect("prerequisite task options"),
        )
        .expect("create prerequisite task");
    head = prerequisite.commit_id;
    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                branch_id,
                head,
                prerequisite.task_entity_id,
                prerequisite.task_entity_version_id,
                "AC-prereq-resource",
                "Prerequisite proof remains recoverable from context.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance criterion options"),
        )
        .expect("create prerequisite acceptance criterion");
    head = criterion.commit_id;
    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                branch_id,
                head,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-prereq-resource",
                "Refresh the prerequisite Resource basis.",
            )
            .expect("verification requirement options"),
        )
        .expect("create prerequisite verification requirement");
    head = requirement.commit_id;

    let (basis, baseline_observation_id) = resource_basis(&mut engine, "docs/prerequisite.md");
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                branch_id,
                head,
                VerificationTarget::VerificationRequirement(
                    requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_resource_basis(vec![basis.clone()])
            .expect("verification resource basis"),
        )
        .expect("create resource-backed prerequisite verification");
    head = verification.commit_id;
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
    engine
        .set_session_focus(SessionFocusOptions::new(
            session.session_id,
            dependent.task_entity_id,
        ))
        .expect("focus session on dependent task");
    let options = ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief);
    let packet = engine
        .context_packet(options.clone())
        .expect("brief context packet");

    assert_eq!(packet.envelope.head_commit_id, relation.commit_id);
    let blocker = packet
        .items
        .iter()
        .find(|item| item.category == ContextItemCategory::BlockedDependency)
        .expect("blocked dependency item");
    assert_eq!(
        blocker.subject.as_ref_string(),
        format!(
            "blocked_dependency:{}:{}",
            dependent.task_entity_id, prerequisite.task_entity_id
        )
    );
    assert!(
        blocker
            .summary
            .contains("dependency_resource_requirements=1")
    );
    assert!(blocker.summary.contains(&format!(
        "dependency_acceptance_criterion={}",
        criterion.acceptance_criterion_entity_id
    )));
    assert!(blocker.summary.contains(&format!(
        "dependency_verification_requirement={}",
        requirement.verification_requirement_entity_id
    )));
    assert!(
        blocker
            .summary
            .contains("dependency_vr_local_key=VR-prereq-resource")
    );
    assert!(blocker.summary.contains("resource_basis=1"));
    assert!(blocker.summary.contains(&format!(
        "verification_id={}",
        verification.verification_entity_id
    )));
    assert!(
        blocker
            .summary
            .contains(&format!("resource_id={}", basis.resource_id))
    );
    assert!(blocker.summary.contains("adapter=local-file@1"));
    assert!(blocker.summary.contains("scope=path@1"));
    assert!(blocker.summary.contains(&format!(
        "baseline_observation_id={baseline_observation_id}"
    )));
    assert!(blocker.summary.contains(&format!(
        "refresh_hint=\"verification cache-refresh --verification {} --resource-content-from-basis\"",
        verification.verification_entity_id
    )));

    let saved = engine
        .save_context_packet(options)
        .expect("save brief context packet");
    let packet_json =
        String::from_utf8(canonical_bytes(&saved.snapshot.packet_json).expect("json"))
            .expect("packet json is utf8");
    assert!(packet_json.contains("\"summary\""));
    assert!(!packet_json.contains("\"resource_basis\":"));
    assert!(!packet_json.contains("\"refresh_hint\":"));
}

#[test]
fn context_packet_summarizes_same_plan_peer_resource_recovery_when_focused_on_task() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let branch_id = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;

    let goal = engine
        .create_goal(
            GoalCreateOptions::new(branch_id, head, "Coordinate same-plan resource recovery")
                .expect("goal options"),
        )
        .expect("create goal");
    head = goal.commit_id;
    let plan = engine
        .create_plan(
            PlanCreateOptions::new(
                branch_id,
                head,
                "Keep peer verification recovery visible",
                "Surface only direct same-plan runnable peer recovery hints",
            )
            .expect("plan options"),
        )
        .expect("create plan");
    head = plan.commit_id;
    let focused = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Focused task without Resource-backed VR")
                .expect("focused task options"),
        )
        .expect("create focused task");
    head = focused.commit_id;
    let peer = engine
        .create_task(
            TaskCreateOptions::new(branch_id, head, "Runnable peer has Resource-backed VR")
                .expect("peer task options"),
        )
        .expect("create peer task");
    head = peer.commit_id;
    let goal_plan = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch_id,
                head,
                goal.goal_entity_id,
                plan.plan_entity_id,
            )
            .expect("goal-plan containment options"),
        )
        .expect("create goal-plan containment");
    head = goal_plan.commit_id;
    let focused_relation = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch_id,
                head,
                plan.plan_entity_id,
                focused.task_entity_id,
            )
            .expect("plan-focused containment options"),
        )
        .expect("create plan-focused containment");
    head = focused_relation.commit_id;
    let peer_relation = engine
        .create_primary_containment(
            PrimaryContainmentCreateOptions::new(
                branch_id,
                head,
                plan.plan_entity_id,
                peer.task_entity_id,
            )
            .expect("plan-peer containment options"),
        )
        .expect("create plan-peer containment");
    head = peer_relation.commit_id;

    let criterion = engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                branch_id,
                head,
                peer.task_entity_id,
                peer.task_entity_version_id,
                "AC-peer-resource",
                "Peer proof remains recoverable from focused context.",
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance criterion options"),
        )
        .expect("create peer acceptance criterion");
    head = criterion.commit_id;
    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                branch_id,
                head,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-peer-resource",
                "Refresh the peer Resource basis.",
            )
            .expect("verification requirement options"),
        )
        .expect("create peer verification requirement");
    head = requirement.commit_id;

    let (basis, baseline_observation_id) = resource_basis(&mut engine, "docs/peer.md");
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                branch_id,
                head,
                VerificationTarget::VerificationRequirement(
                    requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_resource_basis(vec![basis.clone()])
            .expect("verification resource basis"),
        )
        .expect("create peer resource-backed verification");

    let session = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let unfocused_runnable = engine
        .runnable_tasks(RunnableTasksOptions::new(session.session_id))
        .expect("unfocused runnable tasks");
    assert!(
        unfocused_runnable
            .candidates
            .iter()
            .any(
                |candidate| candidate.task.task_entity_id == peer.task_entity_id
                    && candidate.runnable
            )
    );
    engine
        .set_session_focus(SessionFocusOptions::new(
            session.session_id,
            focused.task_entity_id,
        ))
        .expect("focus session on current task");

    let brief = engine
        .context_packet(
            ContextPacketOptions::new(session.session_id).with_profile(ContextProfile::Brief),
        )
        .expect("brief context packet");
    assert!(
        !brief
            .items
            .iter()
            .any(|item| item.summary.contains("same_plan_peer_task="))
    );

    for profile in [ContextProfile::Normal, ContextProfile::Full] {
        let packet = engine
            .context_packet(ContextPacketOptions::new(session.session_id).with_profile(profile))
            .expect("context packet");
        let peer_item = packet
            .items
            .iter()
            .find(|item| {
                item.category == ContextItemCategory::VerificationRequirement
                    && item.summary.contains("local_key=VR-peer-resource")
            })
            .unwrap_or_else(|| {
                panic!("missing same-plan peer VR item in {profile:?}: {packet:#?}")
            });
        assert_eq!(peer_item.priority, ContextPriority::P2);
        assert_eq!(
            peer_item.subject.as_ref_string(),
            format!(
                "verification_requirement:{}",
                requirement.verification_requirement_entity_id
            )
        );
        assert!(
            peer_item
                .summary
                .contains(&format!("same_plan_peer_task={}", peer.task_entity_id))
        );
        assert!(
            peer_item
                .summary
                .contains(&format!("parent_plan={}", plan.plan_entity_id))
        );
        assert!(peer_item.summary.contains("peer_runnable=true"));
        assert!(peer_item.summary.contains(&format!(
            "criterion={}",
            criterion.acceptance_criterion_entity_id
        )));
        assert!(peer_item.summary.contains("resource_basis=1"));
        assert!(peer_item.summary.contains(&format!(
            "verification_id={}",
            verification.verification_entity_id
        )));
        assert!(
            peer_item
                .summary
                .contains(&format!("resource_id={}", basis.resource_id))
        );
        assert!(peer_item.summary.contains("adapter=local-file@1"));
        assert!(peer_item.summary.contains("scope=path@1"));
        assert!(peer_item.summary.contains(&format!(
            "baseline_observation_id={baseline_observation_id}"
        )));
        assert!(peer_item.summary.contains(&format!(
            "refresh_hint=\"verification cache-refresh --verification {} --resource-content-from-basis\"",
            verification.verification_entity_id
        )));
    }
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
    assert!(
        packet
            .omission_summary
            .by_category
            .iter()
            .any(
                |bucket| bucket.category == ContextItemCategory::BlockedDependency
                    && bucket.omitted == 1
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

    let error = ContextPacketOptions::new(SessionId::new_v7())
        .with_scope(CanonicalValue::String("crates/workvcs-core".to_owned()))
        .expect_err("scalar context scope should fail");

    assert_eq!(error.code(), ErrorCode::QueryInvalid);
    assert_eq!(
        error.to_string(),
        "query invalid: context scope must be an object"
    );
}
