use std::collections::BTreeSet;

use serde::Serialize;

const UNKNOWN: &str = "unknown";

/// Precision of the Cargo profile recorded in build provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProfilePrecision {
    Exact,
    ClassOnly,
}

impl BuildProfilePrecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::ClassOnly => "class_only",
        }
    }
}

/// A required build-provenance dimension that is unavailable or incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProvenanceGap {
    PackageVersion,
    SourceMetadata,
    SourceIdentity,
    SourceTreeState,
    Target,
    ProfileClass,
    ProfilePrecision,
    Optimization,
    DebugState,
}

/// Pure assessment of one explicit [`BuildInfo`] value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildProvenanceAssessment {
    UsableCleanExactProfile,
    UsableCleanProfileClassOnly,
    DirtySource {
        profile_precision: BuildProfilePrecision,
    },
    Unavailable {
        gaps: BTreeSet<BuildProvenanceGap>,
    },
}

/// Build provenance and compilation dimensions shared by CLI and MCP surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildInfo {
    pub package_version: &'static str,
    pub git_commit: &'static str,
    pub git_dirty: Option<bool>,
    pub metadata_source: &'static str,
    pub target_triple: &'static str,
    pub build_profile: Option<&'static str>,
    pub profile_class: &'static str,
    pub profile_precision: BuildProfilePrecision,
    pub opt_level: &'static str,
    pub debug: Option<bool>,
    pub build_id: String,
}

impl BuildInfo {
    /// Assesses whether the recorded provenance can identify and reproduce this build.
    pub fn assess_provenance(&self) -> BuildProvenanceAssessment {
        assess_build_provenance(self)
    }

    /// Recomputes the deterministic correlation identity from recorded build facts.
    pub fn deterministic_build_id(&self) -> String {
        let profile = self.build_profile.unwrap_or(UNKNOWN);
        let tree_state = match self.git_dirty {
            Some(true) => "dirty",
            Some(false) => "clean",
            None => UNKNOWN,
        };
        let debug = match self.debug {
            Some(true) => "true",
            Some(false) => "false",
            None => UNKNOWN,
        };
        format!(
            "{};git={};tree={tree_state};metadata_source={};target={};profile={profile};profile_class={};profile_precision={};opt={};debug={debug}",
            self.package_version,
            self.git_commit,
            self.metadata_source,
            self.target_triple,
            self.profile_class,
            self.profile_precision.as_str(),
            self.opt_level,
        )
    }
}

/// Returns the package version and source/compilation dimensions embedded at build time.
pub fn build_info() -> BuildInfo {
    let profile_text = env!("VOLICORD_BUILD_PROFILE");
    let build_profile = (profile_text != UNKNOWN).then_some(profile_text);
    let profile_precision = if build_profile.is_some() {
        BuildProfilePrecision::Exact
    } else {
        BuildProfilePrecision::ClassOnly
    };
    let mut build = BuildInfo {
        package_version: env!("CARGO_PKG_VERSION"),
        git_commit: env!("VOLICORD_BUILD_GIT_COMMIT"),
        git_dirty: parsed_bool(env!("VOLICORD_BUILD_GIT_DIRTY")),
        metadata_source: env!("VOLICORD_BUILD_METADATA_SOURCE"),
        target_triple: env!("VOLICORD_BUILD_TARGET"),
        build_profile,
        profile_class: env!("VOLICORD_BUILD_PROFILE_CLASS"),
        profile_precision,
        opt_level: env!("VOLICORD_BUILD_OPT_LEVEL"),
        debug: parsed_bool(env!("VOLICORD_BUILD_DEBUG")),
        build_id: String::new(),
    };
    build.build_id = build.deterministic_build_id();
    build
}

/// Returns the stable build-correlation identity embedded in this binary.
pub fn build_id() -> String {
    build_info().build_id
}

/// Assesses one explicit build descriptor without reading process or repository state.
pub fn assess_build_provenance(build: &BuildInfo) -> BuildProvenanceAssessment {
    let mut gaps = BTreeSet::new();
    if unknown(build.package_version) {
        gaps.insert(BuildProvenanceGap::PackageVersion);
    }
    if unknown(build.metadata_source) {
        gaps.insert(BuildProvenanceGap::SourceMetadata);
    }
    if unknown(build.git_commit) {
        gaps.insert(BuildProvenanceGap::SourceIdentity);
    }
    if build.git_dirty.is_none() {
        gaps.insert(BuildProvenanceGap::SourceTreeState);
    }
    if unknown(build.target_triple) {
        gaps.insert(BuildProvenanceGap::Target);
    }
    if unknown(build.profile_class) {
        gaps.insert(BuildProvenanceGap::ProfileClass);
    }
    match (build.profile_precision, build.build_profile) {
        (BuildProfilePrecision::Exact, Some(profile)) if !unknown(profile) => {}
        (BuildProfilePrecision::ClassOnly, None) => {}
        _ => {
            gaps.insert(BuildProvenanceGap::ProfilePrecision);
        }
    }
    if unknown(build.opt_level) {
        gaps.insert(BuildProvenanceGap::Optimization);
    }
    if build.debug.is_none() {
        gaps.insert(BuildProvenanceGap::DebugState);
    }
    if !gaps.is_empty() {
        return BuildProvenanceAssessment::Unavailable { gaps };
    }

    if build.git_dirty == Some(true) {
        return BuildProvenanceAssessment::DirtySource {
            profile_precision: build.profile_precision,
        };
    }

    match build.profile_precision {
        BuildProfilePrecision::Exact => BuildProvenanceAssessment::UsableCleanExactProfile,
        BuildProfilePrecision::ClassOnly => BuildProvenanceAssessment::UsableCleanProfileClassOnly,
    }
}

fn unknown(value: &str) -> bool {
    value.is_empty() || value == UNKNOWN
}

fn parsed_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../build_support.rs"]
mod build_support;

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    fn explicit_build() -> BuildInfo {
        let mut build = BuildInfo {
            package_version: "test-package-version",
            git_commit: "0123456789abcdef0123456789abcdef01234567",
            git_dirty: Some(false),
            metadata_source: "repository",
            target_triple: "test-target",
            build_profile: Some("test-profile"),
            profile_class: "test-class",
            profile_precision: BuildProfilePrecision::Exact,
            opt_level: "test-optimization",
            debug: Some(false),
            build_id: String::new(),
        };
        build.build_id = build.deterministic_build_id();
        build
    }

    #[test]
    fn clean_exact_profile_provenance_is_usable() {
        assert_eq!(
            explicit_build().assess_provenance(),
            BuildProvenanceAssessment::UsableCleanExactProfile
        );
    }

    #[test]
    fn clean_class_only_profile_provenance_is_usable() {
        let mut build = explicit_build();
        build.build_profile = None;
        build.profile_precision = BuildProfilePrecision::ClassOnly;
        build.build_id = build.deterministic_build_id();

        assert_eq!(
            build.assess_provenance(),
            BuildProvenanceAssessment::UsableCleanProfileClassOnly
        );
    }

    #[test]
    fn dirty_source_is_a_distinct_reproducibility_limitation() {
        let mut build = explicit_build();
        build.git_dirty = Some(true);
        build.build_profile = None;
        build.profile_precision = BuildProfilePrecision::ClassOnly;

        assert_eq!(
            build.assess_provenance(),
            BuildProvenanceAssessment::DirtySource {
                profile_precision: BuildProfilePrecision::ClassOnly,
            }
        );
    }

    #[test]
    fn unknown_source_metadata_makes_build_identity_unavailable() {
        let mut build = explicit_build();
        build.git_commit = UNKNOWN;
        build.metadata_source = UNKNOWN;

        assert_eq!(
            build.assess_provenance(),
            BuildProvenanceAssessment::Unavailable {
                gaps: BTreeSet::from([
                    BuildProvenanceGap::SourceMetadata,
                    BuildProvenanceGap::SourceIdentity,
                ]),
            }
        );
    }

    #[test]
    fn incomplete_target_or_compilation_metadata_is_unavailable() {
        let mut build = explicit_build();
        build.target_triple = UNKNOWN;
        build.opt_level = UNKNOWN;
        build.debug = None;

        assert_eq!(
            build.assess_provenance(),
            BuildProvenanceAssessment::Unavailable {
                gaps: BTreeSet::from([
                    BuildProvenanceGap::Target,
                    BuildProvenanceGap::Optimization,
                    BuildProvenanceGap::DebugState,
                ]),
            }
        );
    }

    #[test]
    fn build_identity_is_deterministic_and_has_no_time_component() {
        let first = explicit_build();
        let second = explicit_build();
        assert_eq!(first.build_id, second.build_id);
        assert_eq!(first.build_id, first.deterministic_build_id());
        for component in [
            ";git=",
            ";tree=",
            ";metadata_source=",
            ";target=",
            ";profile=",
            ";profile_class=",
            ";profile_precision=",
            ";opt=",
            ";debug=",
        ] {
            assert!(
                first.build_id.contains(component),
                "missing {component}: {}",
                first.build_id
            );
        }
        assert!(!first.build_id.contains("time="));
        assert!(!first.build_id.contains("timestamp="));
        assert!(!first.build_id.contains("built_at="));
    }

    #[test]
    fn embedded_build_identity_matches_embedded_dimensions() {
        let build = build_info();
        assert_eq!(build.build_id, build.deterministic_build_id());
        assert!(!build.package_version.is_empty());
        assert!(!build.git_commit.is_empty());
        assert!(matches!(
            build.metadata_source,
            "repository" | "environment" | "unknown"
        ));
        assert!(!build.target_triple.is_empty());
        assert!(!build.profile_class.is_empty());
        assert!(!build.opt_level.is_empty());
    }

    #[test]
    fn explicit_git_metadata_is_all_or_none_and_strict() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let metadata = build_support::parse_explicit_git_metadata(
            Some(OsString::from(sha)),
            Some(OsString::from("false")),
        )
        .expect("valid metadata")
        .expect("explicit metadata");
        assert_eq!(metadata.commit, sha);
        assert!(!metadata.dirty);

        assert!(
            build_support::parse_explicit_git_metadata(Some(OsString::from(sha)), None).is_err()
        );
        assert!(build_support::parse_explicit_git_metadata(
            Some(OsString::from("not-a-sha")),
            Some(OsString::from("false"))
        )
        .is_err());
        assert!(build_support::parse_explicit_git_metadata(
            Some(OsString::from(sha)),
            Some(OsString::from("unknown"))
        )
        .is_err());
    }

    #[test]
    fn explicit_profile_is_bounded_and_canonical() {
        assert_eq!(
            build_support::parse_explicit_profile(Some(OsString::from("release")))
                .expect("release profile"),
            Some("release".to_owned())
        );
        for invalid in ["", "unknown", "Release", "release profile"] {
            assert!(
                build_support::parse_explicit_profile(Some(OsString::from(invalid))).is_err(),
                "accepted invalid profile: {invalid}"
            );
        }
    }
}
