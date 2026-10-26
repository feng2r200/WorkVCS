use serde_json::{Value, json};
use workvcs_core::CognitionCaptureManifest;

fn valid_manifest() -> Value {
    json!({
        "schema_version": 1,
        "idempotency_key": "capture-validate",
        "records": [{
            "local_id": "finding",
            "kind": "finding",
            "statement": "Validated before any write",
            "scope": {"source": "validation-test"}
        }],
        "knowledge": [],
        "evidence": [],
        "relations": [],
        "rationale": {"mode": "preflight"}
    })
}

fn parse_and_validate(value: &Value) -> workvcs_core::Result<CognitionCaptureManifest> {
    let manifest = CognitionCaptureManifest::from_json_bytes(
        &serde_json::to_vec_pretty(value).expect("manifest JSON"),
    )?;
    manifest.validate_semantics()?;
    Ok(manifest)
}

#[test]
fn valid_manifest_has_a_stable_digest_without_store_access() {
    let first = parse_and_validate(&valid_manifest()).expect("valid manifest");
    let second = parse_and_validate(&valid_manifest()).expect("repeat valid manifest");
    assert_eq!(first.payload_digest(), second.payload_digest());
}

#[test]
fn shape_errors_name_nested_path_position_reason_and_supported_form() {
    let mut value = valid_manifest();
    value["records"][0]["unexpected"] = json!(true);
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    let error = CognitionCaptureManifest::from_json_bytes(&bytes).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("$.records[0]"), "{message}");
    assert!(message.contains("unknown field `unexpected`"), "{message}");
    assert!(
        message.contains("line ") && message.contains("column "),
        "{message}"
    );
    assert!(message.contains("supported form:"), "{message}");

    let root_error = CognitionCaptureManifest::from_json_bytes(b"[]")
        .unwrap_err()
        .to_string();
    assert!(
        root_error.contains("invalid shape at $ (line 1, column 2)"),
        "{root_error}"
    );
    assert!(!root_error.contains("$.."), "{root_error}");
}

#[test]
fn semantic_errors_name_exact_field_reason_and_supported_form() {
    let mut wrong_scope = valid_manifest();
    wrong_scope["records"][0]["scope"] = json!([]);
    let error = parse_and_validate(&wrong_scope).unwrap_err().to_string();
    assert!(error.contains("$.records[0].scope"), "{error}");
    assert!(error.contains("must be an object"), "{error}");
    assert!(error.contains("supported form: a JSON object"), "{error}");

    let mut wrong_kind = valid_manifest();
    wrong_kind["records"][0]["kind"] = json!("milestone");
    let error = parse_and_validate(&wrong_kind).unwrap_err().to_string();
    assert!(error.contains("$.records[0].kind"), "{error}");
    assert!(error.contains("milestone"), "{error}");
    assert!(error.contains("finding"), "{error}");

    let mut wrong_guard = valid_manifest();
    wrong_guard["expected_head_commit_id"] = json!("not-a-commit-id");
    let error = parse_and_validate(&wrong_guard).unwrap_err().to_string();
    assert!(error.contains("$.expected_head_commit_id"), "{error}");
    assert!(error.contains("canonical CommitId"), "{error}");
}

#[test]
fn duplicate_and_relation_errors_name_the_responsible_field() {
    let mut duplicate = valid_manifest();
    duplicate["knowledge"] = json!([{
        "local_id": "finding",
        "statement": "Duplicate id",
        "scope": {},
        "provenance": {}
    }]);
    let error = parse_and_validate(&duplicate).unwrap_err().to_string();
    assert!(error.contains("$.knowledge[0].local_id"), "{error}");
    assert!(error.contains("$.records[0].local_id"), "{error}");

    let mut relation = valid_manifest();
    relation["relations"] = json!([{
        "local_id": "support",
        "type": "supports",
        "source_local_id": "finding",
        "target_local_id": "missing",
        "rationale": "Evidence supports the decision"
    }]);
    let error = parse_and_validate(&relation).unwrap_err().to_string();
    assert!(error.contains("$.relations[0].target_local_id"), "{error}");
    assert!(error.contains("unknown target"), "{error}");
}

#[test]
fn syntax_errors_include_original_line_and_column() {
    let error = CognitionCaptureManifest::from_json_bytes(
        b"{\n  \"schema_version\": 1,\n  \"idempotency_key\": ]\n}\n",
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("line 3, column"), "{error}");
    assert!(error.contains("supported form:"), "{error}");
}
