use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{io::Read, sync::OnceLock};

const UNKNOWN: &str = "unknown";

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
    pub profile_exact: bool,
    pub opt_level: &'static str,
    pub debug: Option<bool>,
    pub build_id: String,
}

/// Returns the package version and source/compilation dimensions embedded at build time.
pub fn build_info() -> BuildInfo {
    let package_version = env!("CARGO_PKG_VERSION");
    let git_commit = env!("VOLICORD_BUILD_GIT_COMMIT");
    let dirty_text = env!("VOLICORD_BUILD_GIT_DIRTY");
    let metadata_source = env!("VOLICORD_BUILD_METADATA_SOURCE");
    let target_triple = env!("VOLICORD_BUILD_TARGET");
    let profile_text = env!("VOLICORD_BUILD_PROFILE");
    let profile_class = env!("VOLICORD_BUILD_PROFILE_CLASS");
    let profile_exact = env!("VOLICORD_BUILD_PROFILE_EXACT") == "true";
    let opt_level = env!("VOLICORD_BUILD_OPT_LEVEL");
    let debug_text = env!("VOLICORD_BUILD_DEBUG");
    let git_dirty = parsed_bool(dirty_text);
    let debug = parsed_bool(debug_text);
    let build_profile = (profile_text != UNKNOWN).then_some(profile_text);
    let tree_state = match git_dirty {
        Some(true) => "dirty",
        Some(false) => "clean",
        None => UNKNOWN,
    };
    let build_id = format!(
        "{package_version};git={git_commit};tree={tree_state};metadata_source={metadata_source};target={target_triple};profile={profile_text};profile_class={profile_class};profile_exact={profile_exact};opt={opt_level};debug={debug_text}"
    );
    BuildInfo {
        package_version,
        git_commit,
        git_dirty,
        metadata_source,
        target_triple,
        build_profile,
        profile_class,
        profile_exact,
        opt_level,
        debug,
        build_id,
    }
}

/// Returns the stable display descriptor embedded in this binary.
pub fn build_id() -> String {
    build_info().build_id
}

pub(crate) fn current_executable_sha256() -> Option<&'static str> {
    static DIGEST: OnceLock<Option<String>> = OnceLock::new();
    DIGEST
        .get_or_init(|| {
            let executable = std::env::current_exe().ok()?;
            let mut file = std::fs::File::open(executable).ok()?;
            let mut hasher = Sha256::new();
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = file.read(&mut buffer).ok()?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Some(format!("{:x}", hasher.finalize()))
        })
        .as_deref()
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

    #[test]
    fn build_identity_has_no_time_component_and_preserves_dimensions() {
        let info = build_info();
        assert!(!info.package_version.is_empty());
        assert!(!info.git_commit.is_empty());
        assert!(matches!(
            info.metadata_source,
            "repository" | "environment" | "unknown"
        ));
        assert!(!info.target_triple.is_empty());
        assert!(!info.profile_class.is_empty());
        assert!(!info.opt_level.is_empty());
        for component in [
            ";git=",
            ";tree=",
            ";metadata_source=",
            ";target=",
            ";profile=",
            ";profile_class=",
            ";profile_exact=",
            ";opt=",
            ";debug=",
        ] {
            assert!(
                info.build_id.contains(component),
                "missing {component}: {}",
                info.build_id
            );
        }
        assert!(!info.build_id.contains("time="));
        assert!(!info.build_id.contains("timestamp="));
        assert!(!info.build_id.contains("built_at="));
    }

    #[test]
    fn current_executable_digest_is_lowercase_sha256() {
        let digest = current_executable_sha256().expect("current test executable must be readable");
        assert_eq!(digest.len(), 64);
        assert!(digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
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
