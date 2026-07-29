use crate::pipeline::CoreResult;
use std::collections::BTreeSet;
use volicord_store::core_pipeline::TaskRecord;
use volicord_types::methods::UpdateScopeRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredScope {
    pub(crate) goal_summary: Option<String>,
    pub(crate) scope_summary: Option<String>,
    pub(crate) non_goals: Vec<String>,
    pub(crate) autonomy_boundary: Option<String>,
    pub(crate) baseline_ref: Option<String>,
}

impl StoredScope {
    pub(crate) fn from_task(task: &TaskRecord) -> CoreResult<Self> {
        Ok(Self::normalized(Self {
            goal_summary: task
                .shaping
                .goal_summary
                .clone()
                .or_else(|| task.summary.clone()),
            scope_summary: task.shaping.scope_summary.clone(),
            non_goals: task.shaping.non_goals.clone(),
            autonomy_boundary: task
                .autonomy_boundary
                .autonomy_boundary
                .clone()
                .or_else(|| task.shaping.autonomy_boundary.clone()),
            baseline_ref: task
                .shaping
                .baseline_ref
                .as_ref()
                .map(|baseline_ref| baseline_ref.as_str().to_owned()),
        }))
    }

    pub(crate) fn apply_request(&self, request: &UpdateScopeRequest) -> Self {
        Self {
            goal_summary: request
                .goal_summary
                .clone()
                .or_else(|| self.goal_summary.clone()),
            scope_summary: request
                .scope_boundary
                .clone()
                .or_else(|| self.scope_summary.clone()),
            non_goals: request
                .non_goals
                .clone()
                .unwrap_or_else(|| self.non_goals.clone()),
            autonomy_boundary: request
                .autonomy_boundary
                .clone()
                .or_else(|| self.autonomy_boundary.clone()),
            baseline_ref: request
                .baseline_ref
                .as_ref()
                .map(|value| value.as_str().to_owned())
                .or_else(|| self.baseline_ref.clone()),
        }
        .normalized()
    }

    fn normalized(mut self) -> Self {
        self.goal_summary = normalize_scope_text_option(self.goal_summary);
        self.scope_summary = normalize_scope_text_option(self.scope_summary);
        self.non_goals = normalize_scope_string_list(self.non_goals);
        self.autonomy_boundary = normalize_scope_text_option(self.autonomy_boundary);
        self.baseline_ref = normalize_scope_text_option(self.baseline_ref);
        self
    }
}

pub(crate) fn normalize_scope_text_option(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub(crate) fn normalize_display_text(value: &str) -> String {
    value.trim().to_owned()
}

pub(crate) fn normalize_scope_string_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| normalize_scope_text_option(Some(value)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
