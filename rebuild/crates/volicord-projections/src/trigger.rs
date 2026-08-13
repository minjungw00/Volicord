use volicord_context::ProjectId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecallTriggerOutcome {
    UnrelatedRequest,
    FirstProjectScoped { project_id: ProjectId },
    LaterProjectScoped { project_id: ProjectId },
}

/// Session-local trigger state. Dropping this value starts a fresh agent
/// session; no canonical record or Candidate is written.
#[derive(Debug, Default)]
pub struct SessionRecallTrigger {
    saw_project_scoped_request: bool,
}

impl SessionRecallTrigger {
    pub const fn new() -> Self {
        Self {
            saw_project_scoped_request: false,
        }
    }

    pub fn observe(&mut self, project_id: Option<ProjectId>) -> RecallTriggerOutcome {
        let Some(project_id) = project_id else {
            return RecallTriggerOutcome::UnrelatedRequest;
        };
        if self.saw_project_scoped_request {
            RecallTriggerOutcome::LaterProjectScoped { project_id }
        } else {
            self.saw_project_scoped_request = true;
            RecallTriggerOutcome::FirstProjectScoped { project_id }
        }
    }
}
