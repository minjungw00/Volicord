use volicord_types::values::{RequestedMode, TaskMode, WorkPhase};

pub(crate) fn resolve_requested_mode(requested_mode: RequestedMode) -> TaskMode {
    match requested_mode {
        RequestedMode::Advisor => TaskMode::Advisor,
        RequestedMode::Direct => TaskMode::Direct,
        RequestedMode::Work | RequestedMode::Auto => TaskMode::Work,
    }
}

pub(crate) fn initial_work_phase(mode: TaskMode) -> WorkPhase {
    match mode {
        TaskMode::Direct => WorkPhase::Implementation,
        TaskMode::Advisor | TaskMode::Work => WorkPhase::Shaping,
    }
}
