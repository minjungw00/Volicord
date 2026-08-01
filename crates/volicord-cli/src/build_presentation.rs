//! Shared human semantics for build provenance.

use volicord_mcp::{BuildProfilePrecision, BuildProvenanceAssessment, BuildProvenanceGap};

use crate::presentation::{HumanValue, YesNo};

pub(crate) const fn source_tree_state(dirty: Option<bool>) -> &'static str {
    match dirty {
        Some(true) => "dirty",
        Some(false) => "clean",
        None => "not recorded",
    }
}

pub(crate) fn metadata_source_state(source: &'static str) -> &'static str {
    match source {
        "repository" => "repository",
        "environment" => "environment",
        "unknown" => "not recorded",
        other => other,
    }
}

pub(crate) const fn profile_precision(precision: BuildProfilePrecision) -> &'static str {
    match precision {
        BuildProfilePrecision::Exact => "exact",
        BuildProfilePrecision::ClassOnly => "class only",
    }
}

pub(crate) fn exact_cargo_profile(profile: Option<&'static str>) -> HumanValue {
    profile
        .map(HumanValue::text)
        .unwrap_or_else(|| HumanValue::text("not recorded"))
}

pub(crate) fn debug_assertions(debug: Option<bool>) -> HumanValue {
    debug
        .map(|value| HumanValue::YesNo(YesNo::from(value)))
        .unwrap_or_else(|| HumanValue::text("not recorded"))
}

pub(crate) const fn provenance_state(assessment: &BuildProvenanceAssessment) -> &'static str {
    match assessment {
        BuildProvenanceAssessment::UsableCleanExactProfile
        | BuildProvenanceAssessment::UsableCleanProfileClassOnly => "usable clean",
        BuildProvenanceAssessment::DirtySource { .. } => "dirty source",
        BuildProvenanceAssessment::Unavailable { .. } => "unavailable",
    }
}

pub(crate) const fn provenance_limitation(
    assessment: &BuildProvenanceAssessment,
) -> Option<&'static str> {
    match assessment {
        BuildProvenanceAssessment::UsableCleanExactProfile
        | BuildProvenanceAssessment::UsableCleanProfileClassOnly => None,
        BuildProvenanceAssessment::DirtySource { .. } => {
            Some("The recorded commit does not identify the working-tree changes.")
        }
        BuildProvenanceAssessment::Unavailable { .. } => {
            Some("Required build provenance is missing or incomplete.")
        }
    }
}

pub(crate) const fn provenance_gap(gap: BuildProvenanceGap) -> &'static str {
    match gap {
        BuildProvenanceGap::PackageVersion => "package version",
        BuildProvenanceGap::SourceMetadata => "source metadata",
        BuildProvenanceGap::SourceIdentity => "source identity",
        BuildProvenanceGap::SourceTreeState => "source tree state",
        BuildProvenanceGap::Target => "target",
        BuildProvenanceGap::ProfileClass => "profile class",
        BuildProvenanceGap::ProfilePrecision => "profile precision",
        BuildProvenanceGap::Optimization => "optimization",
        BuildProvenanceGap::DebugState => "debug state",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_build_states_have_explicit_human_semantics() {
        assert_eq!(profile_precision(BuildProfilePrecision::Exact), "exact");
        assert_eq!(
            profile_precision(BuildProfilePrecision::ClassOnly),
            "class only"
        );
        assert_eq!(source_tree_state(None), "not recorded");
        assert_eq!(metadata_source_state("unknown"), "not recorded");
        assert_eq!(exact_cargo_profile(None).to_string(), "not recorded");
        assert_eq!(debug_assertions(None).to_string(), "not recorded");
    }
}
