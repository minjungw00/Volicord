use crate::ViewerError;
use std::collections::BTreeMap;
use volicord_context::ContextItemId;
use volicord_projections::{ProjectionDetail, WorkSelector};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewerView {
    Overview,
    RepositoryExplanation,
    Work { work: Option<ContextItemId> },
    WorkPage { page: usize },
}
impl ViewerView {
    pub fn named(name: &str) -> Result<Self, ViewerError> {
        match name {
            "overview" => Ok(Self::Overview),
            "repository" => Ok(Self::RepositoryExplanation),
            "work" => Ok(Self::Work { work: None }),
            "tools" | "decisions" | "code" => {
                let mut error = ViewerError::new("retired Viewer view");
                error.not_found = true;
                Err(error)
            }
            _ => Err(ViewerError::new("unknown Viewer view")),
        }
    }
    pub fn parse(values: &BTreeMap<String, String>, default: &Self) -> Result<Self, ViewerError> {
        let value = |key: &str| values.get(key).map(String::as_str);
        let name = value("view").unwrap_or(default.key());
        let mut view = Self::named(name)?;
        for key in ["scope", "entity", "decision", "tool"] {
            if value(key).is_some() {
                return Err(ViewerError::new("retired Viewer selector"));
            }
        }
        if name != "work" && (value("work").is_some() || value("page").is_some()) {
            return Err(ViewerError::new("selector does not belong to this view"));
        }
        if let Some(id) = value("work") {
            if value("page").is_some() {
                return Err(ViewerError::new("detail cannot select a list page"));
            }
            view = match WorkSelector::exact(id) {
                Ok(WorkSelector::ExactWork(id)) => Self::Work { work: Some(id) },
                _ => return Err(ViewerError::new("invalid Work identity")),
            };
        } else if let Some(page) = value("page") {
            let page = if !page.is_empty() && page.bytes().all(|b| b.is_ascii_digit()) {
                page.parse::<usize>().ok().filter(|p| *p < 1_000_000)
            } else {
                None
            }
            .ok_or_else(|| ViewerError::new("invalid list page"))?;
            view = Self::WorkPage { page };
        } else if value("view").is_none() {
            view = default.clone();
        }
        Ok(view)
    }
    pub fn key(&self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::RepositoryExplanation => "repository",
            Self::Work { .. } | Self::WorkPage { .. } => "work",
        }
    }
    pub(crate) fn selection(&self) -> WorkSelector {
        match self {
            Self::Work { work: Some(id) } => WorkSelector::ExactWork(*id),
            Self::RepositoryExplanation => WorkSelector::Repository,
            _ => WorkSelector::LatestWork,
        }
    }
    pub(crate) fn detail(&self) -> ProjectionDetail {
        ProjectionDetail {
            work_page: match self {
                Self::WorkPage { page } => *page,
                _ => 0,
            },
            ..Default::default()
        }
    }
    pub(crate) fn fields(&self) -> Vec<(&'static str, String)> {
        let mut fields = vec![("view", self.key().into())];
        match self {
            Self::WorkPage { page } => fields.push(("page", page.to_string())),
            Self::Work { work: Some(id) } => fields.push(("work", id.to_string())),
            _ => {}
        }
        fields
    }
}
