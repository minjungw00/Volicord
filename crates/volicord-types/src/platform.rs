//! Generic platform identities and canonical path validation.

use std::{error::Error, fmt};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Exact WSL2 distribution name supported by the first release.
pub const PINNED_WSL2_DISTRIBUTION_NAME: &str = "Ubuntu-24.04";

/// Exact WSL2 distribution identifier from `/etc/os-release`.
pub const PINNED_WSL2_DISTRIBUTION_ID: &str = "ubuntu";

/// Exact WSL2 distribution version from `/etc/os-release`.
pub const PINNED_WSL2_DISTRIBUTION_VERSION: &str = "24.04";

const MAX_PLATFORM_PATH_BYTES: usize = 4_096;

/// Operating environment in which Volicord is running.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PlatformEnvironment {
    /// Native Linux.
    Linux,
    /// Native macOS.
    Macos,
    /// Native Windows.
    NativeWindows,
    /// WSL2 with its independent topology requirements.
    Wsl2,
}

impl PlatformEnvironment {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::NativeWindows => "native_windows",
            Self::Wsl2 => "wsl2",
        }
    }
}

/// Stable validation failure for a canonical platform path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformPathError {
    reason: &'static str,
}

impl PlatformPathError {
    /// Returns the stable machine-readable reason.
    pub const fn reason(self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for PlatformPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason)
    }
}

impl Error for PlatformPathError {}

/// Validates one platform path as an already-normalized absolute identity.
pub fn validate_canonical_platform_path(
    platform: PlatformEnvironment,
    path: &str,
) -> Result<(), PlatformPathError> {
    let valid_text =
        !path.is_empty() && path.len() <= MAX_PLATFORM_PATH_BYTES && !path.as_bytes().contains(&0);
    let valid_path = match platform {
        PlatformEnvironment::Linux | PlatformEnvironment::Macos => valid_unix_path(path),
        PlatformEnvironment::Wsl2 => {
            valid_unix_path(path) && path != "/mnt" && !path.starts_with("/mnt/")
        }
        PlatformEnvironment::NativeWindows => valid_native_windows_path(path),
    };
    if valid_text && valid_path {
        Ok(())
    } else {
        Err(PlatformPathError {
            reason: "canonical_platform_path_invalid",
        })
    }
}

fn valid_unix_path(path: &str) -> bool {
    if path == "/" {
        return true;
    }
    if !path.starts_with('/') || path.contains('\\') || path.contains("//") {
        return false;
    }
    if path.len() > 1 && path.ends_with('/') {
        return false;
    }
    path.split('/')
        .skip(1)
        .all(|component| !matches!(component, "" | "." | ".."))
}

fn valid_native_windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_uppercase()
        || bytes[1] != b':'
        || bytes[2] != b'/'
        || path.contains('\\')
        || path[3..].contains("//")
    {
        return false;
    }
    if bytes.len() > 3 && path.ends_with('/') {
        return false;
    }
    path[3..]
        .split('/')
        .all(|component| !matches!(component, "." | "..") && !component.is_empty())
        || bytes.len() == 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_platform_paths_keep_platform_specific_rules() {
        assert!(
            validate_canonical_platform_path(PlatformEnvironment::Linux, "/home/user/project")
                .is_ok()
        );
        assert!(validate_canonical_platform_path(
            PlatformEnvironment::NativeWindows,
            "C:/Users/user/project"
        )
        .is_ok());
        assert!(validate_canonical_platform_path(
            PlatformEnvironment::Wsl2,
            "/mnt/c/Users/user/project"
        )
        .is_err());
        assert!(
            validate_canonical_platform_path(PlatformEnvironment::Wsl2, "/home/user/project")
                .is_ok()
        );
    }
}
