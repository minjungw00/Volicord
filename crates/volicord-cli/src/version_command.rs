//! Product-version and structured build-provenance presentation.

use std::{error::Error, fmt};

use serde::Serialize;
use volicord_command_model::VersionArgs;
use volicord_mcp::BuildInfo;

use crate::{
    build_presentation::{
        debug_assertions, exact_cargo_profile, metadata_source_state, profile_precision,
        source_tree_state,
    },
    presentation::{Document, Field, HumanValue, Section},
};

const PRODUCT_NAME: &str = "Volicord";

/// One typed version report for structured administrative output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionReport {
    pub product_name: &'static str,
    pub package_version: &'static str,
    pub build: BuildInfo,
}

impl VersionReport {
    fn current() -> Self {
        let build = volicord_mcp::build_info();
        Self {
            product_name: PRODUCT_NAME,
            package_version: build.package_version,
            build,
        }
    }
}

#[derive(Debug)]
pub struct VersionCommandError(serde_json::Error);

impl fmt::Display for VersionCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "version report serialization failed: {}", self.0)
    }
}

impl Error for VersionCommandError {}

/// Returns the exact concise product identity used by root version options.
pub fn concise_version() -> String {
    format!("volicord {}\n", env!("CARGO_PKG_VERSION"))
}

/// Renders the explicitly selected version report.
pub fn run_version_command(args: VersionArgs) -> Result<String, VersionCommandError> {
    if args.output.json {
        return serde_json::to_string_pretty(&VersionReport::current())
            .map(|report| format!("{report}\n"))
            .map_err(VersionCommandError);
    }
    if args.output.verbose {
        return Ok(render_verbose_version(&volicord_mcp::build_info()));
    }
    Ok(concise_version())
}

pub(crate) fn render_verbose_version(build: &BuildInfo) -> String {
    Document::verbose(
        format!("{PRODUCT_NAME} {}", build.package_version),
        vec![
            Section::new(
                "Source",
                vec![
                    Field::new("Commit", HumanValue::text(build.git_commit)).into(),
                    Field::new("Tree", HumanValue::text(source_tree_state(build.git_dirty))).into(),
                    Field::new(
                        "Metadata source",
                        HumanValue::text(metadata_source_state(build.metadata_source)),
                    )
                    .into(),
                ],
            )
            .into(),
            Section::new(
                "Build",
                vec![
                    Field::new("Target", HumanValue::text(build.target_triple)).into(),
                    Field::new("Profile class", HumanValue::text(build.profile_class)).into(),
                    Field::new(
                        "Profile precision",
                        HumanValue::text(profile_precision(build.profile_precision)),
                    )
                    .into(),
                    Field::new(
                        "Exact Cargo profile",
                        exact_cargo_profile(build.build_profile),
                    )
                    .into(),
                    Field::new("Optimization", HumanValue::text(build.opt_level)).into(),
                    Field::new("Debug assertions", debug_assertions(build.debug)).into(),
                ],
            )
            .into(),
        ],
    )
    .render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbose_projection_names_profile_precision_without_guessing_exact_profile() {
        let mut build = volicord_mcp::build_info();
        build.build_profile = None;
        build.profile_precision = volicord_mcp::BuildProfilePrecision::ClassOnly;
        build.build_id = build.deterministic_build_id();

        let output = render_verbose_version(&build);
        assert!(output.starts_with(&format!("Volicord {}\n\n", build.package_version)));
        assert!(output.contains("Source\n  Commit:"));
        assert!(output.contains("Build\n  Target:"));
        assert!(output.contains("  Profile class:"));
        assert!(output.contains("  Profile precision: class only"));
        assert!(output.contains("  Exact Cargo profile: not recorded"));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn verbose_projection_shows_an_exact_profile_without_relabeling_it() {
        let mut build = volicord_mcp::build_info();
        build.build_profile = Some("release-with-debug");
        build.profile_precision = volicord_mcp::BuildProfilePrecision::Exact;
        build.build_id = build.deterministic_build_id();

        let output = render_verbose_version(&build);
        assert!(output.contains("  Profile precision: exact"));
        assert!(output.contains("  Exact Cargo profile: release-with-debug"));
    }
}
