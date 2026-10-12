use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use workvcs_core::{
    CognitionCaptureManifest, CognitionCaptureOptions, Engine, ProjectRefId, SearchQueryOptions,
    SearchRangeRole, SearchRefreshOptions, SearchSourceIdentity, StoreInitOptions,
    WorkspaceInitOptions, search_item_context_json, search_lexical_terms,
    search_o200k_ordinary_token_count,
};

#[derive(Default)]
struct Arguments {
    lexical_controls: Option<PathBuf>,
    documents: Option<PathBuf>,
    questions: Option<PathBuf>,
    output: Option<PathBuf>,
    max_results: Option<usize>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = parse_arguments()?;
    let output = if let Some(path) = arguments.lexical_controls.as_deref() {
        if arguments.documents.is_some() || arguments.questions.is_some() {
            return Err("--lexical-controls cannot be combined with corpus evaluation".into());
        }
        run_lexical_controls(path)?
    } else {
        let documents = arguments
            .documents
            .as_deref()
            .ok_or("corpus evaluation requires --documents")?;
        let questions = arguments
            .questions
            .as_deref()
            .ok_or("corpus evaluation requires --questions")?;
        run_reference_evaluation(documents, questions, arguments.max_results)?
    };
    emit_json(&output, arguments.output.as_deref())?;
    Ok(())
}

fn parse_arguments() -> Result<Arguments, Box<dyn std::error::Error>> {
    let mut parsed = Arguments::default();
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || -> Result<String, Box<dyn std::error::Error>> {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} requires a value").into())
        };
        match argument.as_str() {
            "--lexical-controls" => parsed.lexical_controls = Some(value()?.into()),
            "--documents" => parsed.documents = Some(value()?.into()),
            "--questions" => parsed.questions = Some(value()?.into()),
            "--output" => parsed.output = Some(value()?.into()),
            "--max-results" => parsed.max_results = Some(value()?.parse()?),
            "--help" | "-h" => {
                println!(
                    "Usage:\n  fts_reference_harness --lexical-controls FILE [--output FILE]\n  fts_reference_harness --documents FILE --questions FILE [--max-results N] [--output FILE]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument {argument:?}").into()),
        }
    }
    Ok(parsed)
}

fn run_lexical_controls(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let root: Value = serde_json::from_slice(&fs::read(path)?)?;
    let controls = root
        .get("controls")
        .and_then(Value::as_array)
        .ok_or("lexical controls JSON must contain a controls array")?;
    let mut results = Vec::with_capacity(controls.len());
    for control in controls {
        let id = required_string(control, "id")?;
        let text = required_string(control, "text")?;
        results.push(json!({
            "id": id,
            "terms": search_lexical_terms(text),
            "o200k_ordinary_tokens": search_o200k_ordinary_token_count(text),
        }));
    }
    Ok(json!({
        "mode": "lexical_controls",
        "input": path,
        "controls": results,
    }))
}

fn run_reference_evaluation(
    documents_path: &Path,
    questions_path: &Path,
    max_results: Option<usize>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let documents_root: Value = serde_json::from_slice(&fs::read(documents_path)?)?;
    let questions_root: Value = serde_json::from_slice(&fs::read(questions_path)?)?;
    let documents = documents_root
        .get("documents")
        .and_then(Value::as_array)
        .ok_or("documents JSON must contain a documents array")?;
    let questions = questions_root
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("questions JSON must contain a cases array")?;

    let tempdir = tempfile::tempdir()?;
    let source_path = tempdir.path().join("reference-store.sqlite");
    let index_path = tempdir.path().join("reference-search.sqlite");
    let mut engine = Engine::init(
        &source_path,
        StoreInitOptions::new("fts-reference-fixture")?,
    )?;
    let store_id = engine.store_info()?.store_id;
    let workspace = engine.create_workspace(WorkspaceInitOptions::new(
        "fts-reference-fixture-workspace",
    )?)?;
    let project_ref_id = ProjectRefId::new_v7();
    let identity = SearchSourceIdentity::new(
        project_ref_id,
        store_id,
        workspace.workspace_id,
        workspace.initial_branch_id,
    );

    let mut capture_records = Vec::with_capacity(documents.len());
    let mut source_ids_by_local_id = BTreeMap::new();
    for (index, document) in documents.iter().enumerate() {
        let source_id = required_string(document, "id")?;
        let text = required_string(document, "text")?;
        let source_type = document
            .get("source_type")
            .and_then(Value::as_str)
            .unwrap_or("reference_document");
        let local_id = format!("document-{index:06}");
        source_ids_by_local_id.insert(local_id.clone(), source_id.to_owned());
        capture_records.push(json!({
            "local_id": local_id,
            "kind": "finding",
            "statement": text,
            "scope": {
                "fixture_scope": "research_reference_only",
                "input_source_id": source_id,
                "input_source_type": source_type,
            },
        }));
    }
    let manifest_json = serde_json::to_vec(&json!({
        "schema_version": 1,
        "idempotency_key": "fts-reference-fixture-capture-v1",
        "expected_head_commit_id": workspace.genesis_commit_id,
        "expected_state_digest": workspace.state_digest,
        "records": capture_records,
        "knowledge": [],
        "evidence": [],
        "relations": [],
        "rationale": {"scope": "isolated retrieval reference fixture"},
    }))?;
    let manifest = CognitionCaptureManifest::from_json_bytes(&manifest_json)?;
    let captured = engine.capture_cognition(CognitionCaptureOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        workspace.state_digest,
        manifest,
    ))?;
    let mut source_ids = BTreeMap::new();
    for record in captured.records {
        let source_id = source_ids_by_local_id
            .get(&record.local_id)
            .ok_or("capture returned an unknown fixture local id")?;
        if source_ids
            .insert(record.entity_id.to_string(), source_id.clone())
            .is_some()
        {
            return Err("fixture generated a duplicate Record entity id".into());
        }
    }
    drop(engine);

    let readonly = Engine::open_readonly(&source_path)?;
    let refresh_started = Instant::now();
    let refreshed =
        readonly.refresh_search_index(SearchRefreshOptions::new(identity, &index_path))?;
    let refresh_elapsed_us = refresh_started.elapsed().as_micros();
    let mut results = Vec::with_capacity(questions.len());
    for question in questions {
        let id = required_string(question, "id")?;
        let query = evaluation_query(question)?;
        let mut options = SearchQueryOptions::new(identity, &index_path, &query)?;
        if let Some(max_results) = max_results {
            options = options.with_result_limit(max_results)?;
        }
        let started = Instant::now();
        let result = readonly.search(options)?;
        let elapsed_us = started.elapsed().as_micros();
        let items = result
            .items
            .iter()
            .map(|item| {
                let mut item_value = serde_json::to_value(item)
                    .expect("SearchItem serialization must remain infallible");
                let item_object = item_value
                    .as_object_mut()
                    .expect("SearchItem serialization must produce an object");
                item_object.insert(
                    "input_source_id".to_owned(),
                    source_ids
                        .get(&item.entity_id.to_string())
                        .map_or(Value::Null, |source_id| Value::String(source_id.clone())),
                );
                item_object.insert(
                    "item_context_json".to_owned(),
                    Value::String(
                        search_item_context_json(item)
                            .expect("SearchItem context rendering must remain infallible"),
                    ),
                );
                item_object.insert(
                    "canonical_statement_bytes".to_owned(),
                    json!(item.canonical_statement.len()),
                );
                item_object.insert(
                    "query_match_range_bytes".to_owned(),
                    json!(
                        item.ranges
                            .iter()
                            .filter(|range| matches!(
                                range.role,
                                SearchRangeRole::QueryMatch
                                    | SearchRangeRole::QueryMatchAndDocumentLeadContext
                            ))
                            .map(|range| range.snippet_byte_end - range.snippet_byte_start)
                            .sum::<usize>()
                    ),
                );
                item_object.insert(
                    "lead_context_range_bytes".to_owned(),
                    json!(
                        item.ranges
                            .iter()
                            .filter(|range| matches!(
                                range.role,
                                SearchRangeRole::DocumentLeadContext
                                    | SearchRangeRole::QueryMatchAndDocumentLeadContext
                            ))
                            .map(|range| range.snippet_byte_end - range.snippet_byte_start)
                            .sum::<usize>()
                    ),
                );
                item_value
            })
            .collect::<Vec<_>>();
        let mut result_value = serde_json::to_value(&result)?;
        let result_object = result_value
            .as_object_mut()
            .ok_or("SearchQueryResult serialization must produce an object")?;
        result_object.insert("id".to_owned(), Value::String(id.to_owned()));
        result_object.insert("elapsed_us".to_owned(), json!(elapsed_us));
        result_object.insert("items".to_owned(), Value::Array(items));
        results.push(result_value);
    }

    Ok(json!({
        "mode": "isolated_reference_fixture",
        "authority_boundary": "research/reference only; input history is loaded as fresh fixture Records and is not promoted to current project authority",
        "documents_input": documents_path,
        "questions_input": questions_path,
        "documents_loaded": documents.len(),
        "questions_run": questions.len(),
        "max_results": max_results,
        "refresh_elapsed_us": refresh_elapsed_us,
        "refresh": refreshed,
        "results": results,
    }))
}

fn evaluation_query(question: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let query = required_string(question, "query")?;
    let context = match question.get("context") {
        None | Some(Value::Null) => None,
        Some(Value::String(context)) => {
            let context = context.trim();
            (!context.is_empty()).then_some(context)
        }
        Some(_) => return Err("question context must be a string or null".into()),
    };
    Ok(match context {
        Some(context) => format!("{query}\n相关背景：{context}"),
        None => query.to_owned(),
    })
}

fn required_string<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("input item requires string field {field:?}").into())
}

fn emit_json(value: &Value, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = output {
        let mut file = fs::File::create(path)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        writeln!(file)?;
        file.sync_all()?;
    } else {
        let stdout = io::stdout();
        let mut lock = stdout.lock();
        serde_json::to_writer_pretty(&mut lock, value)?;
        writeln!(lock)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_query_matches_the_frozen_context_protocol() {
        let without_context = json!({"query": "问题", "context": null});
        assert_eq!(evaluation_query(&without_context).unwrap(), "问题");

        let blank_context = json!({"query": "问题", "context": " \n\t"});
        assert_eq!(evaluation_query(&blank_context).unwrap(), "问题");

        let with_context = json!({"query": "问题", "context": "  背景事实\n "});
        assert_eq!(
            evaluation_query(&with_context).unwrap(),
            "问题\n相关背景：背景事实"
        );
    }
}
