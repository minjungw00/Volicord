//! Exact published binary target identities used by Codex support contracts.

use std::{error::Error, fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::PlatformEnvironment;

/// One exact Volicord binary target triple.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub enum ReleaseTargetTriple {
    /// GNU/Linux on x86-64.
    #[serde(rename = "x86_64-unknown-linux-gnu")]
    X86_64UnknownLinuxGnu,
    /// GNU/Linux on AArch64.
    #[serde(rename = "aarch64-unknown-linux-gnu")]
    Aarch64UnknownLinuxGnu,
    /// macOS on Apple Silicon.
    #[serde(rename = "aarch64-apple-darwin")]
    Aarch64AppleDarwin,
    /// macOS on Intel x86-64.
    #[serde(rename = "x86_64-apple-darwin")]
    X86_64AppleDarwin,
    /// native Windows using MSVC on x86-64.
    #[serde(rename = "x86_64-pc-windows-msvc")]
    X86_64PcWindowsMsvc,
}

impl ReleaseTargetTriple {
    /// Returns the canonical Rust target-triple spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64UnknownLinuxGnu => "x86_64-unknown-linux-gnu",
            Self::Aarch64UnknownLinuxGnu => "aarch64-unknown-linux-gnu",
            Self::Aarch64AppleDarwin => "aarch64-apple-darwin",
            Self::X86_64AppleDarwin => "x86_64-apple-darwin",
            Self::X86_64PcWindowsMsvc => "x86_64-pc-windows-msvc",
        }
    }

    /// Returns whether this exact target can execute in the environment cell.
    pub const fn supports_environment(self, environment: PlatformEnvironment) -> bool {
        matches!(
            (self, environment),
            (
                Self::X86_64UnknownLinuxGnu,
                PlatformEnvironment::Linux | PlatformEnvironment::Wsl2
            ) | (Self::Aarch64UnknownLinuxGnu, PlatformEnvironment::Linux)
                | (
                    Self::Aarch64AppleDarwin | Self::X86_64AppleDarwin,
                    PlatformEnvironment::Macos
                )
                | (
                    Self::X86_64PcWindowsMsvc,
                    PlatformEnvironment::NativeWindows
                )
        )
    }

    /// Returns the architecture label required from the validating runner.
    pub const fn architecture(self) -> &'static str {
        match self {
            Self::X86_64UnknownLinuxGnu | Self::X86_64AppleDarwin | Self::X86_64PcWindowsMsvc => {
                "x86_64"
            }
            Self::Aarch64UnknownLinuxGnu | Self::Aarch64AppleDarwin => "aarch64",
        }
    }
}

impl fmt::Display for ReleaseTargetTriple {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ReleaseTargetTriple {
    type Err = UnknownReleaseTargetTriple;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "x86_64-unknown-linux-gnu" => Ok(Self::X86_64UnknownLinuxGnu),
            "aarch64-unknown-linux-gnu" => Ok(Self::Aarch64UnknownLinuxGnu),
            "aarch64-apple-darwin" => Ok(Self::Aarch64AppleDarwin),
            "x86_64-apple-darwin" => Ok(Self::X86_64AppleDarwin),
            "x86_64-pc-windows-msvc" => Ok(Self::X86_64PcWindowsMsvc),
            _ => Err(UnknownReleaseTargetTriple),
        }
    }
}

/// Parse failure for an unknown release target triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownReleaseTargetTriple;

impl fmt::Display for UnknownReleaseTargetTriple {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown release target triple")
    }
}

impl Error for UnknownReleaseTargetTriple {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_targets_reject_cross_architecture_and_cross_environment_use() {
        assert!(ReleaseTargetTriple::X86_64UnknownLinuxGnu
            .supports_environment(PlatformEnvironment::Wsl2));
        assert!(!ReleaseTargetTriple::Aarch64UnknownLinuxGnu
            .supports_environment(PlatformEnvironment::Wsl2));
        assert!(!ReleaseTargetTriple::X86_64AppleDarwin
            .supports_environment(PlatformEnvironment::Linux));
        assert!(!ReleaseTargetTriple::Aarch64AppleDarwin
            .supports_environment(PlatformEnvironment::NativeWindows));
    }

    #[test]
    fn all_five_target_spellings_round_trip() {
        for target in [
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
            ReleaseTargetTriple::Aarch64AppleDarwin,
            ReleaseTargetTriple::X86_64AppleDarwin,
            ReleaseTargetTriple::X86_64PcWindowsMsvc,
        ] {
            assert_eq!(target.as_str().parse(), Ok(target));
        }
        assert!("x86_64-unknown-linux-musl"
            .parse::<ReleaseTargetTriple>()
            .is_err());
    }
}
