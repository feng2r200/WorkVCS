//! Explicit applicability selection for persisted work records.

use crate::{
    CanonicalValue, EntityId, PrimaryContainmentSnapshot, Result, StructuralReferenceSnapshot,
    WorkVcsError,
};
use std::collections::BTreeSet;

/// A derived applicability target, independent of the Session's runtime anchor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkApplicabilityTarget {
    ProjectOverview,
    Entity(EntityId),
    ScopedWork,
}

/// One effective selection used before projecting Tasks, records, or runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkApplicability {
    pub target: WorkApplicabilityTarget,
    pub selector: Option<CanonicalValue>,
}

impl WorkApplicability {
    pub fn is_work_scoped(&self) -> bool {
        self.target != WorkApplicabilityTarget::ProjectOverview
    }
    pub fn entity_id(&self) -> Option<EntityId> {
        match self.target {
            WorkApplicabilityTarget::Entity(id) => Some(id),
            _ => None,
        }
    }

    /// None means Project overview; an empty set means explicit No-Plan work.
    /// Callers pass only relations with current authoritative carriers.
    pub fn selected_entities(
        &self,
        relations: &[PrimaryContainmentSnapshot],
        references: &[StructuralReferenceSnapshot],
    ) -> Option<BTreeSet<EntityId>> {
        let mut selected = match self.target {
            WorkApplicabilityTarget::ProjectOverview => return None,
            WorkApplicabilityTarget::ScopedWork => return Some(BTreeSet::new()),
            WorkApplicabilityTarget::Entity(id) => BTreeSet::from([id]),
        };
        loop {
            let before = selected.len();
            for relation in relations {
                if selected.contains(&relation.parent_entity_id) {
                    selected.insert(relation.child_entity_id);
                }
            }
            for reference in references {
                if selected.contains(&reference.referrer_entity_id) {
                    selected.insert(reference.target_entity_id);
                }
            }
            if selected.len() == before {
                return Some(selected);
            }
        }
    }

    /// Entity Focus includes records of descendants reached by current carriers.
    /// Additional selector fields remain conjunctive for those descendants.
    pub fn matches_scope(
        &self,
        scope: &CanonicalValue,
        selected_entities: Option<&BTreeSet<EntityId>>,
    ) -> bool {
        let identity_selection = Self {
            target: self.target.clone(),
            selector: self.selector.as_ref().map(without_path_selectors),
        };
        if !identity_selection.matches_identity_scope(scope, selected_entities) {
            return false;
        }
        let Some(selector) = &self.selector else {
            return true;
        };
        let wanted_paths = PathScopeSelectors::from_scope(selector);
        wanted_paths.is_empty() || wanted_paths.overlaps(&PathScopeSelectors::from_scope(scope))
    }

    fn matches_identity_scope(
        &self,
        scope: &CanonicalValue,
        selected_entities: Option<&BTreeSet<EntityId>>,
    ) -> bool {
        let Some(selector) = &self.selector else {
            return true;
        };
        if work_scope_matches(scope, selector) {
            return true;
        }
        let (CanonicalValue::Object(wanted), CanonicalValue::Object(actual), Some(selected)) =
            (selector, scope, selected_entities)
        else {
            return false;
        };
        if !wanted.iter().any(|(key, _)| key == "focus_entity_id") {
            return false;
        }
        let entity_matches = actual.iter().any(|(key, value)| {
            matches!(key.as_str(), "task_id" | "focus_entity_id")
                && matches!(value, CanonicalValue::String(value) if EntityId::parse_canonical(value).is_ok_and(|id| selected.contains(&id)))
        });
        entity_matches
            && wanted
                .iter()
                .filter(|(key, _)| key != "focus_entity_id")
                .all(|(key, value)| {
                    actual.iter().any(|(candidate, actual)| {
                        candidate == key && work_scope_matches(actual, value)
                    })
                })
    }

    pub fn matches_knowledge_scope(
        &self,
        scope: &CanonicalValue,
        selected_entities: Option<&BTreeSet<EntityId>>,
    ) -> bool {
        let identity_selection = Self {
            target: self.target.clone(),
            selector: self.selector.as_ref().map(without_path_selectors),
        };
        (identity_selection.matches_identity_scope(scope, selected_entities)
            || !has_work_scope_identity(scope)
            || is_project_fact_scope(scope))
            && knowledge_path_scope_matches(scope, self.selector.as_ref())
    }
}

fn without_path_selectors(scope: &CanonicalValue) -> CanonicalValue {
    match scope {
        CanonicalValue::Object(entries) => CanonicalValue::Object(
            entries
                .iter()
                .filter(|(key, _)| {
                    !matches!(
                        key.as_str(),
                        "path" | "paths" | "path_prefix" | "path_prefixes"
                    )
                })
                .filter_map(|(key, value)| {
                    let value = if key == "scope_payload" {
                        without_path_selectors(value)
                    } else {
                        value.clone()
                    };
                    if key == "scope_payload"
                        && matches!(&value, CanonicalValue::Object(entries) if entries.is_empty())
                    {
                        None
                    } else {
                        Some((key.clone(), value))
                    }
                })
                .collect(),
        ),
        value => value.clone(),
    }
}

/// Validate structural work-scope syntax without claiming a Branch membership.
pub fn validate_work_scope(scope: &CanonicalValue) -> Result<()> {
    if !matches!(scope, CanonicalValue::Object(_)) {
        return Err(WorkVcsError::QueryInvalid(
            "work scope must be an object".to_owned(),
        ));
    }
    validate_path_selectors(scope)?;
    let task = scope_entity_id(scope, "task_id")?;
    let focus = scope_entity_id(scope, "focus_entity_id")?;
    if let (Some(task), Some(focus)) = (task, focus)
        && task != focus
    {
        return Err(WorkVcsError::QueryInvalid(
            "explicit work identifiers conflict; select one applicability target".to_owned(),
        ));
    }
    Ok(())
}

/// Normalize explicit work and the existing Session Focus exactly once.
/// A non-path identity without an entity selects No-Plan work. Paths preserve
/// explicit Task/Session Focus, or filter Project overview when neither exists.
pub fn resolve_work_applicability(
    explicit_scope: Option<&CanonicalValue>,
    explicit_task: Option<EntityId>,
    runtime_focus: Option<EntityId>,
    task_ids: &BTreeSet<EntityId>,
) -> Result<WorkApplicability> {
    if let Some(scope) = explicit_scope {
        validate_work_scope(scope)?;
    }
    let identity_scope = explicit_scope.map(without_path_selectors);
    let work_scope = identity_scope
        .as_ref()
        .filter(|scope| matches!(scope, CanonicalValue::Object(entries) if !entries.is_empty()));
    let scope_task = work_scope
        .map(|scope| scope_entity_id(scope, "task_id"))
        .transpose()?
        .flatten();
    let scope_focus = work_scope
        .map(|scope| scope_entity_id(scope, "focus_entity_id"))
        .transpose()?
        .flatten();
    let mut declared = None;
    for id in [explicit_task, scope_task, scope_focus]
        .into_iter()
        .flatten()
    {
        if declared.is_some_and(|prior| prior != id) {
            return Err(WorkVcsError::QueryInvalid(
                "explicit work identifiers conflict; select one applicability target".to_owned(),
            ));
        }
        declared = Some(id);
    }
    for task in [explicit_task, scope_task].into_iter().flatten() {
        if !task_ids.contains(&task) {
            return Err(WorkVcsError::QueryInvalid(format!(
                "selected Task {task} is absent from the selected Branch context"
            )));
        }
    }
    if let (Some(declared), Some(focus)) = (declared, runtime_focus)
        && declared != focus
    {
        return Err(WorkVcsError::QueryInvalid(
            "explicit work target conflicts with Session Focus; change Focus explicitly before continuing".to_owned(),
        ));
    }
    let target_id = declared.or_else(|| work_scope.is_none().then_some(runtime_focus).flatten());
    let target = match target_id {
        Some(id) => WorkApplicabilityTarget::Entity(id),
        None if work_scope.is_some() => WorkApplicabilityTarget::ScopedWork,
        None => WorkApplicabilityTarget::ProjectOverview,
    };
    let mut entries = match explicit_scope {
        Some(CanonicalValue::Object(entries)) => entries.clone(),
        Some(_) => {
            return Err(WorkVcsError::QueryInvalid(
                "work scope must be an object".to_owned(),
            ));
        }
        None => Vec::new(),
    };
    if let Some(id) = target_id {
        let key = if task_ids.contains(&id) {
            "task_id"
        } else {
            "focus_entity_id"
        };
        if !entries.iter().any(|(candidate, _)| candidate == key) {
            entries.push((key.to_owned(), CanonicalValue::String(id.to_string())));
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let selector = (!entries.is_empty()).then_some(CanonicalValue::Object(entries));
    Ok(WorkApplicability { target, selector })
}

fn validate_path_selectors(scope: &CanonicalValue) -> Result<()> {
    let CanonicalValue::Object(entries) = scope else {
        return Ok(());
    };
    for (key, value) in entries {
        let valid = match key.as_str() {
            "path" | "path_prefix" => {
                matches!(value, CanonicalValue::String(value) if !value.is_empty())
            }
            "paths" | "path_prefixes" => {
                matches!(value, CanonicalValue::Array(values) if !values.is_empty() && values.iter().all(|value| matches!(value, CanonicalValue::String(value) if !value.is_empty())))
            }
            "scope_payload" => {
                validate_path_selectors(value)?;
                true
            }
            _ => true,
        };
        if !valid {
            return Err(WorkVcsError::QueryInvalid(format!(
                "work scope field {key} requires nonempty path strings"
            )));
        }
    }
    Ok(())
}

fn scope_entity_id(scope: &CanonicalValue, key: &str) -> Result<Option<EntityId>> {
    let CanonicalValue::Object(entries) = scope else {
        return Err(WorkVcsError::QueryInvalid(
            "work scope must be an object".to_owned(),
        ));
    };
    match entries.iter().find(|(candidate, _)| candidate == key) {
        Some((_, CanonicalValue::String(value))) => EntityId::parse_canonical(value).map(Some),
        Some(_) => Err(WorkVcsError::QueryInvalid(format!(
            "{key} scope must be a canonical entity identifier string"
        ))),
        None => Ok(None),
    }
}

/// Label shared context separately from selected work; neither grants authority.
pub fn work_item_applicability(
    scope: &CanonicalValue,
    selected: bool,
    knowledge: bool,
) -> &'static str {
    if is_project_fact_scope(scope) {
        "project_fact"
    } else if selected {
        "selected_work"
    } else if knowledge && !has_work_scope_identity(scope) {
        "project_knowledge"
    } else {
        "project_overview"
    }
}

/// Match an explicit selector as a structural subset of a record's scope.
///
/// Object keys are conjunctive; arrays and scalar values retain exact canonical
/// equality. This is applicability matching, not natural-language inference.
pub fn work_scope_matches(scope: &CanonicalValue, selector: &CanonicalValue) -> bool {
    match (scope, selector) {
        (CanonicalValue::Object(scope), CanonicalValue::Object(selector)) => {
            selector.iter().all(|(key, wanted)| {
                scope.iter().any(|(candidate, actual)| {
                    candidate == key && work_scope_matches(actual, wanted)
                })
            })
        }
        (actual, wanted) => actual == wanted,
    }
}

/// Return a task selector without introducing another durable work identity.
pub fn task_work_scope(task_id: EntityId) -> CanonicalValue {
    CanonicalValue::Object(vec![(
        "task_id".to_owned(),
        CanonicalValue::String(task_id.to_string()),
    )])
}

/// Identify explicitly published project facts, never an inferred permission.
pub fn is_project_fact_scope(scope: &CanonicalValue) -> bool {
    matches!(scope, CanonicalValue::Object(entries) if entries.iter().any(|(key, value)| {
        key == "applicability" && matches!(value, CanonicalValue::String(value) if value == "project_fact")
    }))
}

/// Whether scope explicitly names work rather than general project knowledge.
pub fn has_work_scope_identity(scope: &CanonicalValue) -> bool {
    matches!(scope, CanonicalValue::Object(entries) if entries.iter().any(|(key, _)| {
        matches!(key.as_str(), "task_id" | "focus_entity_id" | "work" | "session_id")
    }))
}

#[derive(Default)]
struct PathScopeSelectors {
    paths: BTreeSet<String>,
    prefixes: BTreeSet<String>,
}

impl PathScopeSelectors {
    fn from_scope(scope: &CanonicalValue) -> Self {
        let mut selectors = Self::default();
        collect_path_scope_selectors(scope, &mut selectors);
        selectors
    }

    fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.prefixes.is_empty()
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.paths.iter().any(|path| {
            other.paths.contains(path)
                || other
                    .prefixes
                    .iter()
                    .any(|prefix| path_is_within_prefix(path, prefix))
        }) || other.paths.iter().any(|path| {
            self.prefixes
                .iter()
                .any(|prefix| path_is_within_prefix(path, prefix))
        }) || self.prefixes.iter().any(|left| {
            other
                .prefixes
                .iter()
                .any(|right| prefixes_overlap(left, right))
        })
    }
}

fn knowledge_path_scope_matches(
    knowledge_scope: &CanonicalValue,
    context_scope: Option<&CanonicalValue>,
) -> bool {
    let knowledge_selectors = PathScopeSelectors::from_scope(knowledge_scope);
    if knowledge_selectors.is_empty() {
        return true;
    }

    let Some(context_scope) = context_scope else {
        return true;
    };
    let context_selectors = PathScopeSelectors::from_scope(context_scope);
    if context_selectors.is_empty() {
        return true;
    }

    knowledge_selectors.overlaps(&context_selectors)
}

fn collect_path_scope_selectors(scope: &CanonicalValue, selectors: &mut PathScopeSelectors) {
    let CanonicalValue::Object(entries) = scope else {
        return;
    };

    for (key, value) in entries {
        match key.as_str() {
            "path" => collect_string_selector(value, &mut selectors.paths),
            "paths" => collect_string_selectors(value, &mut selectors.paths),
            "path_prefix" => collect_string_selector(value, &mut selectors.prefixes),
            "path_prefixes" => collect_string_selectors(value, &mut selectors.prefixes),
            "scope_payload" => collect_path_scope_selectors(value, selectors),
            _ => {}
        }
    }
}

fn collect_string_selector(value: &CanonicalValue, selectors: &mut BTreeSet<String>) {
    if let CanonicalValue::String(text) = value
        && let Some(selector) = normalize_path_scope_selector(text)
    {
        selectors.insert(selector);
    }
}

fn collect_string_selectors(value: &CanonicalValue, selectors: &mut BTreeSet<String>) {
    if let CanonicalValue::Array(values) = value {
        for value in values {
            collect_string_selector(value, selectors);
        }
    }
}

fn path_is_within_prefix(path: &str, prefix: &str) -> bool {
    if path == prefix {
        return true;
    }
    if prefix.ends_with('/') {
        return path.starts_with(prefix);
    }
    path.strip_prefix(prefix)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn prefixes_overlap(left: &str, right: &str) -> bool {
    path_is_within_prefix(left, right) || path_is_within_prefix(right, left)
}

fn normalize_path_scope_selector(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    let absolute = text.starts_with('/');
    let mut segments = Vec::new();
    for segment in text.split('/') {
        match segment {
            "" | "." => {}
            ".." => match segments.last() {
                Some(last) if *last != ".." => {
                    segments.pop();
                }
                _ if !absolute => segments.push(segment),
                _ => {}
            },
            _ => segments.push(segment),
        }
    }

    if absolute {
        if segments.is_empty() {
            Some("/".to_owned())
        } else {
            Some(format!("/{}", segments.join("/")))
        }
    } else if segments.is_empty() {
        Some(".".to_owned())
    } else {
        Some(segments.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_filter_preserves_selected_focus_and_otherwise_uses_project_overview() {
        let task = EntityId::new_v7();
        let tasks = BTreeSet::from([task]);
        let path = crate::parse_canonical_json(br#"{"path":"src/x.rs"}"#).unwrap();
        let overview = resolve_work_applicability(Some(&path), None, None, &tasks).unwrap();
        assert_eq!(overview.target, WorkApplicabilityTarget::ProjectOverview);
        let focused = resolve_work_applicability(Some(&path), None, Some(task), &tasks).unwrap();
        assert_eq!(focused.target, WorkApplicabilityTarget::Entity(task));
        let explicit = resolve_work_applicability(Some(&path), Some(task), None, &tasks).unwrap();
        assert_eq!(explicit.target, WorkApplicabilityTarget::Entity(task));
        let named =
            crate::parse_canonical_json(br#"{"work":"named-work","path":"src/x.rs"}"#).unwrap();
        let no_plan = resolve_work_applicability(Some(&named), None, Some(task), &tasks).unwrap();
        assert_eq!(no_plan.target, WorkApplicabilityTarget::ScopedWork);
    }

    #[test]
    fn work_scope_syntax_does_not_claim_task_membership() {
        let task = EntityId::new_v7();
        let scope =
            crate::parse_canonical_json(format!(r#"{{"task_id":"{task}"}}"#).as_bytes()).unwrap();
        validate_work_scope(&scope).unwrap();
        assert!(resolve_work_applicability(Some(&scope), None, None, &BTreeSet::new()).is_err());
        let malformed = crate::parse_canonical_json(br#"{"task_id":7}"#).unwrap();
        assert!(validate_work_scope(&malformed).is_err());
        let empty_paths = crate::parse_canonical_json(br#"{"paths":[]}"#).unwrap();
        assert!(validate_work_scope(&empty_paths).is_err());
    }
}
