use std::fmt;
use volicord_context::{
    CanonicalReadBasis, CheckpointId, ContextItemId, ContextItemRole, ProjectId,
};

/// Read-side scope; independent of HTTP view names and canonical mutation APIs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WorkSelector {
    #[default]
    LatestWork,
    ExactWork(ContextItemId),
    Repository,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkSelectionBasis {
    ExactGoal {
        revision: u64,
    },
    LatestCheckpoint {
        checkpoint_id: CheckpointId,
        revision: u64,
    },
    LatestGoal {
        revision: u64,
    },
    UnassociatedCheckpoint {
        checkpoint_id: CheckpointId,
        revision: u64,
    },
    NoWork,
    Repository,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkSelection {
    pub selector: WorkSelector,
    pub work_item_id: Option<ContextItemId>,
    pub basis: WorkSelectionBasis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkSelectionError {
    InvalidIdentity,
    WorkNotFound {
        project_id: ProjectId,
        work_item_id: ContextItemId,
    },
}

impl fmt::Display for WorkSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity => {
                f.write_str("Work identity must be exactly 32 hexadecimal digits")
            }
            Self::WorkNotFound {
                project_id,
                work_item_id,
            } => write!(
                f,
                "Work {work_item_id} is not an available Goal in Project {project_id}"
            ),
        }
    }
}
impl std::error::Error for WorkSelectionError {}

impl WorkSelector {
    /// Parse only the existing canonical identity representation; no aliases.
    pub fn exact(identity: &str) -> Result<Self, WorkSelectionError> {
        if identity.len() != 32 || !identity.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(WorkSelectionError::InvalidIdentity);
        }
        let mut bytes = [0; 16];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&identity[index * 2..index * 2 + 2], 16)
                .map_err(|_| WorkSelectionError::InvalidIdentity)?;
        }
        Ok(Self::ExactWork(ContextItemId::from_bytes(bytes)))
    }

    pub fn resolve(
        self,
        canonical: &CanonicalReadBasis,
    ) -> Result<WorkSelection, WorkSelectionError> {
        let goal = |identity| {
            canonical.context_items.iter().find(|item| {
                item.id == identity
                    && item.project_id == canonical.project.id
                    && item.role == ContextItemRole::Goal
            })
        };
        let (work_item_id, basis) = match self {
            Self::Repository => (None, WorkSelectionBasis::Repository),
            Self::ExactWork(identity) => {
                let item = goal(identity).ok_or(WorkSelectionError::WorkNotFound {
                    project_id: canonical.project.id,
                    work_item_id: identity,
                })?;
                (
                    Some(identity),
                    WorkSelectionBasis::ExactGoal {
                        revision: item.revision,
                    },
                )
            }
            Self::LatestWork => {
                let latest = canonical
                    .checkpoint_history
                    .iter()
                    .chain(canonical.latest_checkpoint.iter())
                    .filter(|cp| cp.project_id == canonical.project.id)
                    .max_by_key(|cp| (cp.recorded_at, cp.id));
                if let Some(cp) = latest {
                    match cp.work_item_id.filter(|id| goal(*id).is_some()) {
                        Some(id) => (
                            Some(id),
                            WorkSelectionBasis::LatestCheckpoint {
                                checkpoint_id: cp.id,
                                revision: cp.revision,
                            },
                        ),
                        None => (
                            None,
                            WorkSelectionBasis::UnassociatedCheckpoint {
                                checkpoint_id: cp.id,
                                revision: cp.revision,
                            },
                        ),
                    }
                } else if let Some(item) = canonical
                    .context_items
                    .iter()
                    .filter(|item| {
                        item.project_id == canonical.project.id
                            && item.role == ContextItemRole::Goal
                    })
                    .max_by_key(|item| (item.recorded_at, item.id))
                {
                    (
                        Some(item.id),
                        WorkSelectionBasis::LatestGoal {
                            revision: item.revision,
                        },
                    )
                } else {
                    (None, WorkSelectionBasis::NoWork)
                }
            }
        };
        Ok(WorkSelection {
            selector: self,
            work_item_id,
            basis,
        })
    }
}
