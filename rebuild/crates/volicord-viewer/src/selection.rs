use crate::ViewerError;
use std::collections::BTreeMap;
use volicord_context::ContextItemId;
use volicord_projections::{ProjectionDetail, WorkSelector};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewerView {
    Overview,
    WorkQuery {
        query: volicord_projections::WorkCatalogueQuery,
    },
    RepositoryExplanation,
    WorkContext {
        work: ContextItemId,
        query: volicord_projections::WorkCatalogueQuery,
    },
    Work {
        work: Option<ContextItemId>,
    },
    WorkPage {
        page: usize,
    },
    WorkFilter {
        state: volicord_projections::UnderstandingWorkState,
        page: usize,
    },
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
        let has_query = ["q", "state", "sort", "page", "basis"]
            .iter()
            .any(|key| value(key).is_some());
        if name != "work" && (value("work").is_some() || has_query || value("from").is_some()) {
            return Err(ViewerError::new("selector does not belong to this view"));
        }
        let work = value("work")
            .map(|id| match WorkSelector::exact(id) {
                Ok(WorkSelector::ExactWork(id)) => Ok(id),
                _ => Err(ViewerError::new("invalid Work identity")),
            })
            .transpose()?;
        let from_list = match value("from") {
            None => false,
            Some("list") if work.is_some() => true,
            _ => return Err(ViewerError::new("invalid Work origin")),
        };
        if work.is_some() && has_query && !from_list {
            return Err(ViewerError::new("detail query requires a list origin"));
        }
        if name == "work" {
            let page = match value("page") {
                None => 0,
                Some(page) if !page.is_empty() && page.bytes().all(|b| b.is_ascii_digit()) => page
                    .parse::<usize>()
                    .ok()
                    .filter(|p| *p < 1_000_000)
                    .ok_or_else(|| ViewerError::new("invalid list page"))?,
                _ => return Err(ViewerError::new("invalid list page")),
            };
            let state = match value("state") {
                None => None,
                Some(s) => Some(match s {
                    "open" => volicord_projections::UnderstandingWorkState::Open,
                    "in_progress" => volicord_projections::UnderstandingWorkState::InProgress,
                    "paused" => volicord_projections::UnderstandingWorkState::Paused,
                    "completed" => volicord_projections::UnderstandingWorkState::Completed,
                    "abandoned" => volicord_projections::UnderstandingWorkState::Abandoned,
                    "superseded" => volicord_projections::UnderstandingWorkState::Superseded,
                    _ => return Err(ViewerError::new("unknown Work state")),
                }),
            };
            let order = match value("sort").unwrap_or("recent") {
                "recent" => volicord_projections::WorkCatalogueOrder::Recent,
                "oldest" => volicord_projections::WorkCatalogueOrder::Oldest,
                "name" => volicord_projections::WorkCatalogueOrder::Name,
                _ => return Err(ViewerError::new("unknown Work ordering")),
            };
            let query = volicord_projections::WorkCatalogueQuery {
                term: value("q").unwrap_or_default().into(),
                state,
                order,
                page,
                expected_basis: value("basis").map(str::to_owned),
            };
            query.validate().map_err(ViewerError::new)?;
            view = match work {
                Some(work) if from_list => Self::WorkContext { work, query },
                Some(work) => Self::Work { work: Some(work) },
                None => Self::WorkQuery { query },
            };
        } else if value("view").is_none() {
            view = default.clone();
        }
        Ok(view)
    }
    pub fn key(&self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::RepositoryExplanation => "repository",
            Self::WorkContext { .. }
            | Self::WorkQuery { .. }
            | Self::Work { .. }
            | Self::WorkPage { .. }
            | Self::WorkFilter { .. } => "work",
        }
    }
    pub(crate) fn exact_work(&self) -> Option<ContextItemId> {
        match self {
            Self::Work { work: Some(id) } | Self::WorkContext { work: id, .. } => Some(*id),
            _ => None,
        }
    }
    pub(crate) fn list_return(&self) -> Self {
        match self {
            Self::WorkContext { query, .. } => Self::WorkQuery {
                query: query.clone(),
            },
            _ => Self::Work { work: None },
        }
    }
    pub(crate) fn catalogue_query(&self) -> Option<volicord_projections::WorkCatalogueQuery> {
        let mut query = volicord_projections::WorkCatalogueQuery::default();
        match self {
            Self::Overview | Self::Work { work: None } => {}
            Self::WorkPage { page } => query.page = *page,
            Self::WorkFilter { state, page } => {
                query.state = Some(*state);
                query.page = *page;
            }
            Self::WorkQuery { query } => return Some(query.clone()),
            _ => return None,
        }
        Some(query)
    }
    pub(crate) fn selection(&self) -> WorkSelector {
        match self {
            Self::Work { work: Some(id) } | Self::WorkContext { work: id, .. } => {
                WorkSelector::ExactWork(*id)
            }
            Self::RepositoryExplanation => WorkSelector::Repository,
            _ => WorkSelector::LatestWork,
        }
    }
    pub(crate) fn detail(&self) -> ProjectionDetail {
        ProjectionDetail {
            work_page: match self {
                Self::WorkPage { page } | Self::WorkFilter { page, .. } => *page,
                _ => 0,
            },
            work_state: match self {
                Self::WorkQuery { query } => query.state,
                Self::WorkFilter { state, .. } => Some(*state),
                _ => None,
            },
            ..Default::default()
        }
    }
    pub(crate) fn fields(&self) -> Vec<(&'static str, String)> {
        if let Self::WorkContext { work, query } = self {
            let mut fields = Self::WorkQuery {
                query: query.clone(),
            }
            .fields();
            fields.push(("work", work.to_string()));
            fields.push(("from", "list".into()));
            return fields;
        }
        let mut fields = vec![("view", self.key().into())];
        match self {
            Self::WorkQuery { query } => {
                fields.push(("q", query.term.clone()));
                fields.push(("page", query.page.to_string()));
                fields.push((
                    "sort",
                    match query.order {
                        volicord_projections::WorkCatalogueOrder::Recent => "recent",
                        volicord_projections::WorkCatalogueOrder::Oldest => "oldest",
                        volicord_projections::WorkCatalogueOrder::Name => "name",
                    }
                    .into(),
                ));
                if let Some(state) = query.state {
                    fields.push(("state", super::render::work_state_key(state).into()));
                }
                if let Some(basis) = &query.expected_basis {
                    fields.push(("basis", basis.clone()));
                }
            }
            Self::WorkPage { page } => fields.push(("page", page.to_string())),
            Self::WorkFilter { state, page } => {
                fields.push(("page", page.to_string()));
                fields.push(("state", super::render::work_state_key(*state).into()));
            }
            Self::Work { work: Some(id) } => fields.push(("work", id.to_string())),
            _ => {}
        }
        fields
    }
}
