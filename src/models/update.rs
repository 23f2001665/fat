use std::collections::BTreeMap;

use semver::Version;
use serde::{Deserialize, Serialize};

/// Central update configuration. It is read from the app data directory and
/// can also be pointed at a mirror without recompiling.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// HTTPS URL of the JSON release manifest. Empty disables update checks.
    #[serde(default)]
    pub release_source: String,
    /// Hex-encoded Ed25519 publisher key. When set, signatures are mandatory.
    #[serde(default)]
    pub public_key_hex: Option<String>,
    /// Optional local installer command for desktop package managers.
    ///
    /// `{package}` is replaced with the absolute path of the verified package.
    #[serde(default)]
    pub installer_command: Option<String>,
    /// Allow `file://` sources for air-gapped installations. Disabled by default.
    #[serde(default)]
    pub allow_file_urls: bool,
    /// Optional SHA-256 pin for the manifest itself, protecting against a
    /// compromised release host redirecting to another publisher.
    #[serde(default)]
    pub manifest_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseArtifact {
    pub url: String,
    /// Lowercase hex SHA-256 of the package.
    pub sha256: String,
    /// Optional hex Ed25519 signature over the package bytes.
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// Optional file name to preserve an installer-friendly extension.
    #[serde(default)]
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseMetadata {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub released_at: Option<String>,
    /// Oldest database schema on which this release can run.
    pub schema_min: i64,
    /// Newest database schema this release understands.
    pub schema_max: i64,
    /// Platform identifier (android-arm64, linux-x86_64, ...) to artifact.
    #[serde(default)]
    pub artifacts: BTreeMap<String, ReleaseArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub released_at: Option<String>,
    pub schema_min: i64,
    pub schema_max: i64,
    #[serde(default)]
    pub artifacts: BTreeMap<String, ReleaseArtifact>,
    /// Optional signed package of the previously supported release. Used for
    /// an explicit, user-initiated rollback where the platform permits it.
    #[serde(default)]
    pub rollback: Option<ReleaseMetadata>,
}

impl ReleaseManifest {
    pub fn version(&self) -> Result<Version, semver::Error> {
        parse_version(&self.version)
    }

    pub fn artifact_for_current_platform(&self) -> Option<&ReleaseArtifact> {
        self.artifacts.get(current_platform())
    }
}

impl ReleaseMetadata {
    pub fn version(&self) -> Result<Version, semver::Error> {
        parse_version(&self.version)
    }
}

pub fn parse_version(value: &str) -> Result<Version, semver::Error> {
    Version::parse(value.trim().trim_start_matches('v'))
}

pub fn current_platform() -> &'static str {
    #[cfg(all(target_os = "android", target_arch = "aarch64"))]
    {
        return "android-arm64";
    }
    #[cfg(all(target_os = "android", target_arch = "x86_64"))]
    {
        return "android-x86_64";
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return "linux-x86_64";
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return "linux-aarch64";
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return "macos-aarch64";
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return "macos-x86_64";
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return "windows-x86_64";
    }
    #[allow(unreachable_code)]
    "unknown"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateAvailability {
    NotConfigured,
    UpToDate,
    Available,
    IncompatibleDatabase,
    UnsupportedPlatform,
    AlreadyNewer,
}

impl UpdateAvailability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotConfigured => "Update source not configured",
            Self::UpToDate => "Up to date",
            Self::Available => "Update available",
            Self::IncompatibleDatabase => "Incompatible database schema",
            Self::UnsupportedPlatform => "No package for this platform",
            Self::AlreadyNewer => "Installed build is newer than the release channel",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateCheck {
    pub availability: UpdateAvailability,
    pub current_version: Version,
    pub latest_version: Option<Version>,
    pub notes: String,
    pub released_at: Option<String>,
    pub schema_min: Option<i64>,
    pub schema_max: Option<i64>,
    pub has_artifact: bool,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StagedPackage {
    pub version: String,
    pub path: String,
    pub sha256: String,
    pub verified: bool,
    pub downloaded_at: String,
    /// Release metadata captured at download time, used to re-verify before install.
    #[serde(default)]
    pub metadata_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateState {
    pub last_checked_at: Option<String>,
    pub latest_version: Option<String>,
    pub latest_notes: String,
    pub latest_released_at: Option<String>,
    pub staged: Option<StagedPackage>,
    pub rollback_staged: Option<StagedPackage>,
    pub last_install_started_at: Option<String>,
    pub last_install_package: Option<String>,
    pub last_install_version: Option<String>,
    pub pre_install_backup: Option<String>,
}

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    Progress { downloaded: u64, total: Option<u64> },
    Status(String),
}

#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub staged: StagedPackage,
    pub path: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct InstallOutcome {
    pub message: String,
    pub installer_launched: bool,
    pub package_path: String,
    pub backup_path: Option<String>,
}
