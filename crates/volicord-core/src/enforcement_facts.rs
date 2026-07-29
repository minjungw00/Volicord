use crate::pipeline::{CorePipelineError, CoreResult};
use volicord_store::core_pipeline::CoreProjectStore;
use volicord_types::schema::ProjectEnforcementProfile;

pub(crate) fn project_enforcement_profile(
    store: &CoreProjectStore,
) -> CoreResult<ProjectEnforcementProfile> {
    Ok(store
        .project_enforcement_profile()
        .map_err(CorePipelineError::from)?
        .profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforcement_fact_owner_exposes_a_typed_store_read() {
        let _: fn(&CoreProjectStore<'_>) -> CoreResult<ProjectEnforcementProfile> =
            project_enforcement_profile;
    }
}
