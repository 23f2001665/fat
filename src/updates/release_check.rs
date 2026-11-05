use std::io::Read;
use std::path::Path;
use std::time::Duration;

use crate::error::{AppError, AppResult};
use crate::models::update::{parse_version, ReleaseArtifact, ReleaseManifest, UpdateConfig};
use crate::updates::verification::{checksum_matches, sha256_bytes, validate_sha256};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024;

fn file_path_from_url(url: &str) -> AppResult<&Path> {
    let path = url
        .strip_prefix("file://")
        .ok_or_else(|| AppError::update("Invalid file URL"))?;
    if path.is_empty() {
        return Err(AppError::update("Empty file URL"));
    }
    Ok(Path::new(path))
}

pub fn validate_source_url(url: &str, allow_file_urls: bool) -> AppResult<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::update("Update source is not configured"));
    }
    if trimmed.starts_with("https://") {
        return Ok(());
    }
    if allow_file_urls && trimmed.starts_with("file://") {
        return Ok(());
    }
    if trimmed.starts_with("http://") {
        return Err(AppError::update(
            "Plain HTTP update sources are refused; use HTTPS or an explicit file:// source",
        ));
    }
    Err(AppError::update(
        "Update source must be an HTTPS URL (or file:// when explicitly enabled)",
    ))
}

pub fn read_source_bytes(config: &UpdateConfig, url: &str, max_bytes: u64) -> AppResult<Vec<u8>> {
    validate_source_url(url, config.allow_file_urls)?;
    if url.starts_with("file://") {
        let path = file_path_from_url(url)?;
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > max_bytes {
            return Err(AppError::update("Local update file is unexpectedly large"));
        }
        return Ok(std::fs::read(path)?);
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(45))
        .build();
    let response = agent
        .get(url)
        .call()
        .map_err(|err| AppError::network(format!("Could not reach the update source: {err}")))?;
    if let Some(length) = response.header("Content-Length") {
        if let Ok(length) = length.parse::<u64>() {
            if length > max_bytes {
                return Err(AppError::update("Update metadata is unexpectedly large"));
            }
        }
    }
    let mut reader = std::io::Read::take(response.into_reader(), max_bytes + 1);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(AppError::update("Update metadata is unexpectedly large"));
    }
    Ok(bytes)
}

pub fn parse_manifest(bytes: &[u8]) -> AppResult<ReleaseManifest> {
    let manifest: ReleaseManifest = serde_json::from_slice(bytes)
        .map_err(|err| AppError::update(format!("Release metadata is invalid JSON: {err}")))?;
    manifest
        .version()
        .map_err(|err| AppError::update(format!("Release version is not valid semver: {err}")))?;
    if manifest.schema_min < 1 || manifest.schema_min > manifest.schema_max {
        return Err(AppError::update(
            "Release metadata has an invalid database schema compatibility range",
        ));
    }
    let mut names = std::collections::HashSet::new();
    for (platform, artifact) in &manifest.artifacts {
        if platform.trim().is_empty() {
            return Err(AppError::update(
                "Release artifact has an empty platform name",
            ));
        }
        if !names.insert(platform) {
            return Err(AppError::update("Duplicate release platform entry"));
        }
        validate_artifact(artifact)?;
    }
    if let Some(rollback) = &manifest.rollback {
        rollback
            .version()
            .map_err(|err| AppError::update(format!("Rollback version is invalid: {err}")))?;
        if rollback.schema_min < 1 || rollback.schema_min > rollback.schema_max {
            return Err(AppError::update(
                "Rollback metadata has an invalid schema compatibility range",
            ));
        }
        for artifact in rollback.artifacts.values() {
            validate_artifact(artifact)?;
        }
    }
    Ok(manifest)
}

pub fn validate_artifact(artifact: &ReleaseArtifact) -> AppResult<()> {
    if artifact.url.trim().is_empty() {
        return Err(AppError::update("Release artifact URL is empty"));
    }
    validate_sha256(&artifact.sha256)?;
    if let Some(signature) = &artifact.signature {
        if signature.len() != 128 || !signature.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AppError::update(
                "Release signature must be 128 hexadecimal characters",
            ));
        }
    }
    if let Some(size) = artifact.size_bytes {
        if size == 0 || size > MAX_PACKAGE_BYTES {
            return Err(AppError::update("Release artifact size is invalid"));
        }
    }
    if let Some(file_name) = &artifact.file_name {
        if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
            return Err(AppError::update("Release artifact file name is unsafe"));
        }
    }
    Ok(())
}

pub fn fetch_manifest(config: &UpdateConfig) -> AppResult<ReleaseManifest> {
    if config.release_source.trim().is_empty() {
        return Err(AppError::update(
            "No update source is configured. Add a release source to the update configuration.",
        ));
    }
    let bytes = read_source_bytes(config, &config.release_source, MAX_MANIFEST_BYTES)?;
    if let Some(expected) = &config.manifest_sha256 {
        let actual = sha256_bytes(&bytes);
        if !checksum_matches(expected, &actual) {
            return Err(AppError::update(
                "Release metadata checksum does not match the pinned SHA-256",
            ));
        }
    }
    let manifest = parse_manifest(&bytes)?;
    if manifest.artifact_for_current_platform().is_none() {
        // Not fatal: the UI can still show the version and notes. The service
        // reports `UnsupportedPlatform` when an update would otherwise be offered.
    }
    Ok(manifest)
}

/// Parses and validates a release manifest from a local file (for air-gapped
/// installations and tests).
pub fn read_manifest_file(path: &Path) -> AppResult<ReleaseManifest> {
    let bytes = std::fs::read(path)?;
    parse_manifest(&bytes)
}

pub fn platform_artifact(manifest: &ReleaseManifest) -> Option<&ReleaseArtifact> {
    manifest.artifact_for_current_platform()
}

pub fn versions_compare(remote: &str, current: &semver::Version) -> AppResult<std::cmp::Ordering> {
    let remote = parse_version(remote)
        .map_err(|err| AppError::update(format!("Release version is invalid: {err}")))?;
    Ok(remote.cmp(current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::update::ReleaseArtifact;

    fn sample_manifest(version: &str) -> ReleaseManifest {
        let mut artifacts = std::collections::BTreeMap::new();
        artifacts.insert(
            crate::models::update::current_platform().to_string(),
            ReleaseArtifact {
                url: "https://example.com/fat.apk".into(),
                sha256: "a".repeat(64),
                signature: None,
                size_bytes: Some(123),
                file_name: Some("fat.apk".into()),
            },
        );
        ReleaseManifest {
            version: version.into(),
            notes: "notes".into(),
            released_at: None,
            schema_min: 1,
            schema_max: 2,
            artifacts,
            rollback: None,
        }
    }

    #[test]
    fn manifest_requires_valid_version_and_schema_range() {
        let valid = serde_json::to_vec(&sample_manifest("1.2.3")).unwrap();
        parse_manifest(&valid).unwrap();

        let mut invalid = sample_manifest("not-semver");
        let bytes = serde_json::to_vec(&invalid).unwrap();
        assert!(parse_manifest(&bytes).is_err());

        invalid = sample_manifest("1.2.3");
        invalid.schema_min = 3;
        invalid.schema_max = 2;
        let bytes = serde_json::to_vec(&invalid).unwrap();
        assert!(parse_manifest(&bytes).is_err());
    }

    #[test]
    fn insecure_sources_are_rejected() {
        assert!(validate_source_url("http://example.com/releases.json", false).is_err());
        assert!(validate_source_url("file:///tmp/manifest.json", false).is_err());
        assert!(validate_source_url("file:///tmp/manifest.json", true).is_ok());
        assert!(validate_source_url("https://example.com/releases.json", false).is_ok());
    }

    #[test]
    fn unsafe_artifact_file_name_is_rejected() {
        let mut manifest = sample_manifest("1.2.3");
        manifest
            .artifacts
            .get_mut(crate::models::update::current_platform())
            .unwrap()
            .file_name = Some("../evil.apk".into());
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert!(parse_manifest(&bytes).is_err());
    }

    #[test]
    fn version_comparison_uses_semver() {
        let current = semver::Version::parse("1.4.0").unwrap();
        assert_eq!(
            versions_compare("1.4.1", &current).unwrap(),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            versions_compare("1.3.9", &current).unwrap(),
            std::cmp::Ordering::Less
        );
    }
}
