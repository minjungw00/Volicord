use crate::ViewerError;
use std::collections::BTreeMap;
use volicord_context::{ContextItemId, DecisionId};
use volicord_projections::{ProjectionDetail, WorkSelector};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeScope {
    Work(Option<ContextItemId>),
    Repository,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewerTool {
    Documents,
    Memory,
    Status,
    Evidence,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewerView {
    WorkPage {
        page: usize,
    },
    DecisionsPage {
        page: usize,
    },
    Overview,
    Work {
        work: Option<ContextItemId>,
    },
    Code {
        scope: CodeScope,
        entity: Option<String>,
    },
    Decisions {
        decision: Option<DecisionId>,
    },
    Tools {
        tool: ViewerTool,
    },
}
impl ViewerView {
    pub fn named(name: &str) -> Result<Self, ViewerError> {
        match name {
            "overview" => Ok(Self::Overview),
            "work" => Ok(Self::Work { work: None }),
            "code" => Ok(Self::Code {
                scope: CodeScope::Repository,
                entity: None,
            }),
            "decisions" => Ok(Self::Decisions { decision: None }),
            "tools" => Ok(Self::Tools {
                tool: ViewerTool::Status,
            }),
            _ => Err(ViewerError::new("unknown Viewer view")),
        }
    }
    pub fn parse(values: &BTreeMap<String, String>, default: &Self) -> Result<Self, ViewerError> {
        let value = |key: &str| values.get(key).map(String::as_str);
        let name = value("view").unwrap_or(default.key());
        let allowed: &[&str] = match name {
            "overview" => &[],
            "work" => &["work", "page"],
            "code" => &["scope", "work", "entity"],
            "decisions" => &["decision", "page"],
            "tools" => &["tool"],
            _ => return Err(ViewerError::new("unknown Viewer view")),
        };
        for key in ["scope", "work", "entity", "decision", "tool", "page"] {
            if value(key).is_some() && !allowed.contains(&key) {
                return Err(ViewerError::new("selector does not belong to this view"));
            }
        }
        let page = value("page")
            .map(|p| {
                if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(ViewerError::new("invalid list page"));
                }
                p.parse::<usize>()
                    .ok()
                    .filter(|p| *p < 1_000_000)
                    .ok_or_else(|| ViewerError::new("invalid list page"))
            })
            .transpose()?
            .unwrap_or(0);
        if value("page").is_some() && (value("work").is_some() || value("decision").is_some()) {
            return Err(ViewerError::new("detail cannot select a list page"));
        }
        let work = || {
            value("work")
                .map(|id| match WorkSelector::exact(id) {
                    Ok(WorkSelector::ExactWork(id)) => Ok(id),
                    _ => Err(ViewerError::new(
                        "Work identity must contain 32 hexadecimal digits",
                    )),
                })
                .transpose()
        };
        Ok(match name {
            "overview" => Self::Overview,
            "work" if page > 0 => Self::WorkPage { page },
            "work" => Self::Work { work: work()? },
            "code" => {
                let scope = match value("scope") {
                    Some("work") => {
                        CodeScope::Work(Some(work()?.ok_or_else(|| {
                            ViewerError::new("Work code requires a Work identity")
                        })?))
                    }
                    Some("repository") => {
                        if value("work").is_some() {
                            return Err(ViewerError::new("repository scope cannot select a Work"));
                        }
                        CodeScope::Repository
                    }
                    None if value("view").is_none() => match default {
                        Self::Code { scope, .. } => scope.clone(),
                        _ => CodeScope::Repository,
                    },
                    _ => return Err(ViewerError::new("code scope must be work or repository")),
                };
                let entity = value("entity")
                    .map(|id| {
                        if id.is_empty() || id.len() > 1024 || id.chars().any(char::is_control) {
                            Err(ViewerError::new("invalid entity identity"))
                        } else {
                            Ok(id.to_owned())
                        }
                    })
                    .transpose()?;
                Self::Code { scope, entity }
            }
            "decisions" if page > 0 => Self::DecisionsPage { page },
            "decisions" => Self::Decisions {
                decision: value("decision")
                    .map(|id| match WorkSelector::exact(id) {
                        Ok(WorkSelector::ExactWork(id)) => {
                            Ok(DecisionId::from_bytes(*id.as_bytes()))
                        }
                        _ => Err(ViewerError::new("invalid Decision identity")),
                    })
                    .transpose()?,
            },
            "tools" => Self::Tools {
                tool: match value("tool").unwrap_or("status") {
                    "documents" => ViewerTool::Documents,
                    "memory" => ViewerTool::Memory,
                    "status" => ViewerTool::Status,
                    "evidence" => ViewerTool::Evidence,
                    _ => return Err(ViewerError::new("unknown Viewer tool")),
                },
            },
            _ => unreachable!(),
        })
    }
    pub fn key(&self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Work { .. } | Self::WorkPage { .. } => "work",
            Self::Code { .. } => "code",
            Self::Decisions { .. } | Self::DecisionsPage { .. } => "decisions",
            Self::Tools { .. } => "tools",
        }
    }
    pub(crate) fn selection(&self) -> WorkSelector {
        match self {
            Self::Work { work: Some(id) }
            | Self::Code {
                scope: CodeScope::Work(Some(id)),
                ..
            } => WorkSelector::ExactWork(*id),
            Self::Code {
                scope: CodeScope::Repository,
                ..
            }
            | Self::Decisions { .. } => WorkSelector::Repository,
            _ => WorkSelector::LatestWork,
        }
    }
    pub(crate) fn detail(&self) -> ProjectionDetail {
        ProjectionDetail {
            work_page: match self {
                Self::WorkPage { page } => *page,
                _ => 0,
            },
            decision_page: match self {
                Self::DecisionsPage { page } => *page,
                _ => 0,
            },
            decision: match self {
                Self::Decisions { decision } => *decision,
                _ => None,
            },
            entity: match self {
                Self::Code { entity, .. } => entity.clone(),
                _ => None,
            },
        }
    }
    pub(crate) fn fields(&self) -> Vec<(&'static str, String)> {
        let mut fields = vec![("view", self.key().into())];
        match self {
            Self::WorkPage { page } | Self::DecisionsPage { page } => {
                fields.push(("page", page.to_string()))
            }
            Self::Work { work: Some(id) } => fields.push(("work", id.to_string())),
            Self::Code { scope, entity } => {
                match scope {
                    CodeScope::Repository => fields.push(("scope", "repository".into())),
                    CodeScope::Work(id) => {
                        fields.push(("scope", "work".into()));
                        if let Some(id) = id {
                            fields.push(("work", id.to_string()));
                        }
                    }
                }
                if let Some(id) = entity {
                    fields.push(("entity", id.clone()));
                }
            }
            Self::Decisions { decision: Some(id) } => fields.push(("decision", id.to_string())),
            Self::Tools { tool } => fields.push((
                "tool",
                match tool {
                    ViewerTool::Documents => "documents",
                    ViewerTool::Memory => "memory",
                    ViewerTool::Status => "status",
                    ViewerTool::Evidence => "evidence",
                }
                .into(),
            )),
            _ => {}
        }
        fields
    }
}
