use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;
use semver::Version;

use crate::database::backup::{create_backup, verify_integrity};
use crate::database::connection::ensure_foreign_keys;
use crate::error::{AppError, AppResult};
use crate::models::update::{
    current_platform, parse_version, DownloadOutcome, InstallOutcome, ReleaseArtifact,
    ReleaseManifest, ReleaseMetadata, StagedPackage, UpdateAvailability, UpdateCheck, UpdateConfig,
    UpdateState,
};
use crate::updates::download::download_package;
use crate::updates::release_check::{fetch_manifest, validate_source_url};
use crate::updates::verification::{parse_public_key, verify_package};
use crate::utils::date::now_timestamp;

pub struct UpdateService {
    data_dir: PathBuf,
    db_path: PathBuf,
    config: UpdateConfig,
    current_version: Version,
    current_schema: i64,
    state: UpdateState,
}

impl UpdateService {
    pub fn new(
        data_dir: impl AsRef<Path>,
        db_path: impl AsRef<Path>,
        current_schema: i64,
    ) -> AppResult<Self> {
        let version = parse_version(env!("CARGO_PKG_VERSION"))
            .map_err(|err| AppError::update(format!("Invalid application version: {err}")))?;
        Self::with_version(data_dir, db_path, current_schema, version)
    }

    pub fn with_version(
        data_dir: impl AsRef<Path>,
        db_path: impl AsRef<Path>,
        current_schema: i64,
        current_version: Version,
    ) -> AppResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(data_dir.join("updates").join("packages"))?;
        let config_path = data_dir.join("updates").join("config.json");
        let config = if config_path.exists() {
            let bytes = std::fs::read(&config_path)?;
            serde_json::from_slice(&bytes).map_err(|err| {
                AppError::update(format!(
                    "Update configuration {} is invalid: {err}",
                    config_path.display()
                ))
            })?
        } else {
            let config = UpdateConfig::default();
            atomic_write_json(&config_path, &config)?;
            config
        };
        let state_path = data_dir.join("updates").join("state.json");
        let state = if state_path.exists() {
            let bytes = std::fs::read(&state_path)?;
            serde_json::from_slice(&bytes).unwrap_or_else(|err| {
                eprintln!(
                    "Ignoring unreadable update state ({}): {err}",
                    state_path.display()
                );
                UpdateState::default()
            })
        } else {
            UpdateState::default()
        };
        let db_path = if db_path.as_ref().is_absolute() {
            db_path.as_ref().to_path_buf()
        } else {
            std::env::current_dir()?.join(db_path.as_ref())
        };

        Ok(Self {
            data_dir,
            db_path,
            config,
            current_version,
            current_schema,
            state,
        })
    }

    pub fn current_version(&self) -> &Version {
        &self.current_version
    }

    pub fn current_schema(&self) -> i64 {
        self.current_schema
    }

    pub fn state(&self) -> &UpdateState {
        &self.state
    }

    pub fn config(&self) -> &UpdateConfig {
        &self.config
    }

    pub fn config_path(&self) -> PathBuf {
        self.data_dir.join("updates").join("config.json")
    }

    pub fn state_path(&self) -> PathBuf {
        self.data_dir.join("updates").join("state.json")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.data_dir.join("updates").join("latest-manifest.json")
    }

    fn updates_dir(&self) -> PathBuf {
        self.data_dir.join("updates")
    }

    fn save_state(&self) -> AppResult<()> {
        atomic_write_json(&self.state_path(), &self.state)
    }

    pub fn reload_config(&mut self) -> AppResult<()> {
        let path = self.config_path();
        let bytes = std::fs::read(&path)?;
        self.config = serde_json::from_slice(&bytes).map_err(|err| {
            AppError::update(format!(
                "Update configuration {} is invalid: {err}",
                path.display()
            ))
        })?;
        if let Some(key) = &self.config.public_key_hex {
            parse_public_key(key)?;
        }
        Ok(())
    }

    pub fn check(&mut self) -> AppResult<UpdateCheck> {
        self.reload_config()?;
        let current = self.current_version.clone();
        if self.config.release_source.trim().is_empty() {
            return Ok(UpdateCheck {
                availability: UpdateAvailability::NotConfigured,
                current_version: current,
                latest_version: None,
                notes: format!(
                    "Add a release source to {} to enable update checks.",
                    self.config_path().display()
                ),
                released_at: None,
                schema_min: None,
                schema_max: None,
                has_artifact: false,
                rollback_available: false,
            });
        }
        validate_source_url(&self.config.release_source, self.config.allow_file_urls)?;
        if let Some(key) = &self.config.public_key_hex {
            parse_public_key(key)?;
        }

        let manifest = fetch_manifest(&self.config)?;
        let latest = manifest
            .version()
            .map_err(|err| AppError::update(format!("Invalid release version: {err}")))?;
        let has_artifact = manifest.artifact_for_current_platform().is_some();
        let schema_compatible = schema_compatible(
            self.current_schema,
            manifest.schema_min,
            manifest.schema_max,
        );
        let availability = match latest.cmp(&current) {
            Ordering::Greater if !schema_compatible => UpdateAvailability::IncompatibleDatabase,
            Ordering::Greater if !has_artifact => UpdateAvailability::UnsupportedPlatform,
            Ordering::Greater => UpdateAvailability::Available,
            Ordering::Equal if !schema_compatible => UpdateAvailability::IncompatibleDatabase,
            Ordering::Equal => UpdateAvailability::UpToDate,
            Ordering::Less => UpdateAvailability::AlreadyNewer,
        };
        let rollback_available = rollback_is_compatible(&manifest, &current, self.current_schema);

        self.state.last_checked_at = Some(now_timestamp());
        self.state.latest_version = Some(latest.to_string());
        self.state.latest_notes = manifest.notes.clone();
        self.state.latest_released_at = manifest.released_at.clone();
        self.save_state()?;
        atomic_write_json(&self.manifest_path(), &manifest)?;

        Ok(UpdateCheck {
            availability,
            current_version: current,
            latest_version: Some(latest),
            notes: manifest.notes.clone(),
            released_at: manifest.released_at.clone(),
            schema_min: Some(manifest.schema_min),
            schema_max: Some(manifest.schema_max),
            has_artifact,
            rollback_available,
        })
    }

    pub fn latest_manifest(&self) -> AppResult<ReleaseManifest> {
        let path = self.manifest_path();
        if !path.exists() {
            return Err(AppError::update(
                "No release metadata cached. Check for updates first.",
            ));
        }
        let bytes = std::fs::read(path)?;
        let manifest: ReleaseManifest = serde_json::from_slice(&bytes)?;
        Ok(manifest)
    }

    fn ensure_schema_supported(&self, metadata: &ReleaseMetadata) -> AppResult<()> {
        if !schema_compatible(
            self.current_schema,
            metadata.schema_min,
            metadata.schema_max,
        ) {
            return Err(AppError::update(format!(
                "Release {} supports database schema {}–{}, but this installation uses schema {}. \
                 A database backup is required before changing versions.",
                metadata.version, metadata.schema_min, metadata.schema_max, self.current_schema
            )));
        }
        Ok(())
    }

    fn save_release_metadata(
        &self,
        metadata: &ReleaseMetadata,
        version: &Version,
    ) -> AppResult<PathBuf> {
        let safe_version = safe_version_component(version);
        let path = self
            .updates_dir()
            .join("packages")
            .join(format!("{safe_version}-release.json"));
        atomic_write_json(&path, metadata)?;
        Ok(path)
    }

    pub fn download_latest(
        &mut self,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> AppResult<DownloadOutcome> {
        self.reload_config()?;
        let manifest = self.latest_manifest()?;
        let metadata = metadata_from_manifest(&manifest);
        let latest = metadata
            .version()
            .map_err(|err| AppError::update(format!("Invalid release version: {err}")))?;
        if latest <= self.current_version {
            return Err(AppError::update(
                "The available release is not newer than the installed version",
            ));
        }
        self.ensure_schema_supported(&metadata)?;
        let artifact = metadata
            .artifacts
            .get(current_platform())
            .ok_or_else(|| {
                AppError::update(format!(
                    "Release {} has no package for {}",
                    metadata.version,
                    current_platform()
                ))
            })?
            .clone();
        let mut outcome = self.download_artifact(&metadata, &artifact, progress)?;
        let metadata_path = self.save_release_metadata(&metadata, &latest)?;
        outcome.staged.metadata_path = Some(metadata_path.to_string_lossy().into_owned());
        self.state.staged = Some(outcome.staged.clone());
        self.save_state()?;
        Ok(outcome)
    }

    pub fn download_rollback(
        &mut self,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> AppResult<DownloadOutcome> {
        self.reload_config()?;
        let manifest = self.latest_manifest()?;
        let rollback = manifest
            .rollback
            .as_ref()
            .ok_or_else(|| {
                AppError::update("The release channel does not provide a rollback package")
            })?
            .clone();
        let version = rollback
            .version()
            .map_err(|err| AppError::update(format!("Invalid rollback version: {err}")))?;
        if version >= self.current_version {
            return Err(AppError::update(
                "The rollback package is not older than the installed version",
            ));
        }
        self.ensure_schema_supported(&rollback)?;
        let artifact = rollback
            .artifacts
            .get(current_platform())
            .ok_or_else(|| {
                AppError::update(format!(
                    "Rollback release {} has no package for {}",
                    version,
                    current_platform()
                ))
            })?
            .clone();
        let mut outcome = self.download_artifact(&rollback, &artifact, progress)?;
        let metadata_path = self.save_release_metadata(&rollback, &version)?;
        outcome.staged.metadata_path = Some(metadata_path.to_string_lossy().into_owned());
        self.state.rollback_staged = Some(outcome.staged.clone());
        self.save_state()?;
        Ok(outcome)
    }

    fn download_artifact(
        &self,
        metadata: &ReleaseMetadata,
        artifact: &ReleaseArtifact,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> AppResult<DownloadOutcome> {
        let version = metadata
            .version()
            .map_err(|err| AppError::update(format!("Invalid release version: {err}")))?;
        let extension = package_extension(artifact)?;
        let package_dir = self
            .updates_dir()
            .join("packages")
            .join(safe_version_component(&version));
        std::fs::create_dir_all(&package_dir)?;
        let final_path = package_dir.join(format!("{}.{extension}", current_platform()));
        let partial_path = package_dir.join(format!("{}.{extension}.part", current_platform()));

        let download = download_package(
            &self.config,
            &artifact.url,
            &partial_path,
            artifact.size_bytes,
            progress,
        )?;
        if !crate::updates::verification::checksum_matches(&artifact.sha256, &download.sha256) {
            let _ = std::fs::remove_file(&partial_path);
            return Err(AppError::update(
                "Package checksum does not match trusted release metadata",
            ));
        }
        if let Err(error) = verify_package(
            &partial_path,
            artifact,
            self.config.public_key_hex.as_deref(),
        ) {
            let _ = std::fs::remove_file(&partial_path);
            return Err(error);
        }
        if final_path.exists() {
            std::fs::remove_file(&final_path)?;
        }
        std::fs::rename(&partial_path, &final_path)?;
        let path = std::fs::canonicalize(&final_path).unwrap_or(final_path);
        Ok(DownloadOutcome {
            staged: StagedPackage {
                version: version.to_string(),
                path: path.to_string_lossy().into_owned(),
                sha256: download.sha256,
                verified: true,
                downloaded_at: now_timestamp(),
                metadata_path: None,
            },
            path,
        })
    }

    /// Backs up the live database and re-verifies the staged package before
    /// handing it to a configured installer. The application is never silently
    /// replaced; without an installer command the package is left staged and
    /// the user is told where it is.
    pub fn install_staged(&mut self, rollback: bool) -> AppResult<InstallOutcome> {
        let staged = if rollback {
            self.state.rollback_staged.clone()
        } else {
            self.state.staged.clone()
        }
        .ok_or_else(|| {
            AppError::update(if rollback {
                "No rollback package has been downloaded"
            } else {
                "No update package has been downloaded"
            })
        })?;
        if !staged.verified {
            return Err(AppError::update(
                "Refusing to install an unverified package",
            ));
        }
        let package_path = PathBuf::from(&staged.path);
        if !package_path.exists() {
            return Err(AppError::update(
                "The staged package is no longer present; download it again",
            ));
        }
        let metadata = self.load_staged_metadata(&staged)?;
        let artifact = metadata
            .artifacts
            .get(current_platform())
            .ok_or_else(|| AppError::update("Staged release has no package for this platform"))?;
        // Re-verify against the trust metadata captured at download time. This
        // detects tampering that occurred while the package sat on disk.
        verify_package(
            &package_path,
            artifact,
            self.config.public_key_hex.as_deref(),
        )?;
        self.ensure_schema_supported(&metadata)?;

        let backup_path = self.backup_database(&staged.version)?;
        self.state.last_install_started_at = Some(now_timestamp());
        self.state.last_install_package = Some(staged.path.clone());
        self.state.last_install_version = Some(staged.version.clone());
        self.state.pre_install_backup = Some(backup_path.to_string_lossy().into_owned());
        self.save_state()?;

        let message = if let Some(template) = &self.config.installer_command {
            launch_installer(template, &package_path)?;
            format!(
                "Installer launched for {} {}. Database backup: {}.",
                if rollback { "rollback to" } else { "update to" },
                staged.version,
                backup_path.display()
            )
        } else if cfg!(target_os = "android") {
            format!(
                "Verified package staged at {}. Use the Android system package installer \
                 (the host activity must provide a FileProvider content URI). Android may refuse \
                 a version-code downgrade. Database backup: {}.",
                package_path.display(),
                backup_path.display()
            )
        } else {
            format!(
                "Verified package staged at {}. No installer command is configured, so the \
                 application was not modified. Database backup: {}.",
                package_path.display(),
                backup_path.display()
            )
        };

        Ok(InstallOutcome {
            installer_launched: self.config.installer_command.is_some(),
            message,
            package_path: staged.path,
            backup_path: Some(backup_path.to_string_lossy().into_owned()),
        })
    }

    fn load_staged_metadata(&self, staged: &StagedPackage) -> AppResult<ReleaseMetadata> {
        let path = staged.metadata_path.as_ref().ok_or_else(|| {
            AppError::update("Staged package is missing its release metadata; download it again")
        })?;
        let bytes = std::fs::read(path)?;
        let metadata: ReleaseMetadata = serde_json::from_slice(&bytes)?;
        let metadata_version = metadata
            .version()
            .map_err(|err| AppError::update(format!("Invalid staged release version: {err}")))?;
        let staged_version = parse_version(&staged.version)
            .map_err(|err| AppError::update(format!("Invalid staged package version: {err}")))?;
        if metadata_version != staged_version {
            return Err(AppError::update(
                "Staged package metadata does not match its version",
            ));
        }
        Ok(metadata)
    }

    fn backup_database(&self, target_version: &str) -> AppResult<PathBuf> {
        let conn = Connection::open(&self.db_path)?;
        conn.busy_timeout(Duration::from_secs(10))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        ensure_foreign_keys(&conn)?;
        verify_integrity(&conn)?;
        let label = format!(
            "pre-update-v{}-to-v{}",
            self.current_version, target_version
        );
        create_backup(&conn, &self.updates_dir().join("backups"), &label)
    }

    pub fn pre_install_backup(&self) -> Option<&str> {
        self.state.pre_install_backup.as_deref()
    }
}

fn schema_compatible(current_schema: i64, min: i64, max: i64) -> bool {
    current_schema >= min && current_schema <= max
}

fn rollback_is_compatible(
    manifest: &ReleaseManifest,
    current_version: &Version,
    current_schema: i64,
) -> bool {
    let Some(rollback) = &manifest.rollback else {
        return false;
    };
    let Ok(version) = rollback.version() else {
        return false;
    };
    version < *current_version
        && schema_compatible(current_schema, rollback.schema_min, rollback.schema_max)
        && rollback.artifacts.contains_key(current_platform())
}

fn metadata_from_manifest(manifest: &ReleaseManifest) -> ReleaseMetadata {
    ReleaseMetadata {
        version: manifest.version.clone(),
        notes: manifest.notes.clone(),
        released_at: manifest.released_at.clone(),
        schema_min: manifest.schema_min,
        schema_max: manifest.schema_max,
        artifacts: manifest.artifacts.clone(),
    }
}

fn safe_version_component(version: &Version) -> String {
    version
        .to_string()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '+') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn package_extension(artifact: &ReleaseArtifact) -> AppResult<String> {
    let candidate = artifact.file_name.as_deref().unwrap_or_else(|| {
        artifact
            .url
            .split('?')
            .next()
            .unwrap_or(&artifact.url)
            .rsplit('/')
            .next()
            .unwrap_or("")
    });
    let extension = candidate
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or("");
    if extension.is_empty() {
        return Ok("bin".into());
    }
    if extension.len() > 8 || !extension.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(AppError::update(
            "Package URL has an unsafe or unsupported file extension",
        ));
    }
    Ok(extension.to_ascii_lowercase())
}

fn launch_installer(template: &str, package: &Path) -> AppResult<()> {
    if !template.contains("{package}") {
        return Err(AppError::update(
            "installer_command must contain the {package} placeholder",
        ));
    }
    let package = package.to_string_lossy();
    #[cfg(unix)]
    let command = {
        let quoted = format!("'{}'", package.replace('\'', "'\\''"));
        template.replace("{package}", &quoted)
    };
    #[cfg(windows)]
    let command = {
        let quoted = format!("\"{}\"", package.replace('"', "\"\""));
        template.replace("{package}", &quoted)
    };

    #[cfg(unix)]
    let mut process = std::process::Command::new("sh");
    #[cfg(unix)]
    process.arg("-c").arg(command);
    #[cfg(windows)]
    let mut process = std::process::Command::new("cmd");
    #[cfg(windows)]
    process.arg("/C").arg(command);

    process
        .spawn()
        .map_err(|err| AppError::update(format!("Could not launch installer: {err}")))?;
    Ok(())
}

fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(&temp, bytes)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::updates::verification::sha256_bytes;
    use ed25519_dalek::{Signer, SigningKey};
    use std::collections::BTreeMap;

    struct TestContext {
        _dir: tempfile::TempDir,
        db_path: PathBuf,
        artifact_bytes: Vec<u8>,
        service: UpdateService,
    }

    fn artifact_dir(dir: &Path) -> PathBuf {
        let path = dir.join("downloads");
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[allow(clippy::too_many_arguments)]
    fn write_manifest(
        dir: &Path,
        data_dir: &Path,
        version: &str,
        schema_min: i64,
        schema_max: i64,
        artifact_path: &Path,
        signature: Option<String>,
        public_key: Option<String>,
    ) -> PathBuf {
        let bytes = std::fs::read(artifact_path).unwrap();
        let mut artifacts = BTreeMap::new();
        artifacts.insert(
            current_platform().to_string(),
            ReleaseArtifact {
                url: format!("file://{}", artifact_path.display()),
                sha256: sha256_bytes(&bytes),
                signature,
                size_bytes: Some(bytes.len() as u64),
                file_name: Some("fat-test.apk".into()),
            },
        );
        let manifest = ReleaseManifest {
            version: version.into(),
            notes: "Test release notes".into(),
            released_at: Some("2026-11-05T00:00:00Z".into()),
            schema_min,
            schema_max,
            artifacts,
            rollback: None,
        };
        let path = dir.join("manifest.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let updates_dir = data_dir.join("updates");
        std::fs::create_dir_all(&updates_dir).unwrap();
        let config = UpdateConfig {
            release_source: format!("file://{}", path.display()),
            public_key_hex: public_key,
            installer_command: None,
            allow_file_urls: true,
            manifest_sha256: None,
        };
        std::fs::write(
            updates_dir.join("config.json"),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        path
    }

    fn context(version: &str, schema_min: i64, schema_max: i64) -> TestContext {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("app-data");
        let db_path = data_dir.join("workout.db");
        std::fs::create_dir_all(&data_dir).unwrap();
        Database::open(&db_path).unwrap();
        let artifact_bytes = b"fake signed apk bytes".to_vec();
        let artifact_path = artifact_dir(dir.path()).join("fat-test.apk");
        std::fs::write(&artifact_path, &artifact_bytes).unwrap();
        write_manifest(
            dir.path(),
            &data_dir,
            version,
            schema_min,
            schema_max,
            &artifact_path,
            None,
            None,
        );
        let service =
            UpdateService::with_version(&data_dir, &db_path, 1, Version::parse("0.1.0").unwrap())
                .unwrap();
        TestContext {
            _dir: dir,
            db_path,
            artifact_bytes,
            service,
        }
    }

    #[test]
    fn missing_source_is_reported_without_network_access() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        let db_path = data_dir.join("db.sqlite");
        Database::open(&db_path).unwrap();
        let mut service =
            UpdateService::with_version(&data_dir, &db_path, 1, Version::parse("0.1.0").unwrap())
                .unwrap();
        let check = service.check().unwrap();
        assert_eq!(check.availability, UpdateAvailability::NotConfigured);
        assert!(!check.has_artifact);
    }

    #[test]
    fn check_and_download_from_local_manifest() {
        let mut ctx = context("0.2.0", 1, 1);
        let check = ctx.service.check().unwrap();
        assert_eq!(check.availability, UpdateAvailability::Available);
        assert_eq!(check.latest_version, Some(Version::parse("0.2.0").unwrap()));
        assert!(check.has_artifact);

        let mut progress_seen = 0;
        let outcome = ctx
            .service
            .download_latest(&mut |done, _| progress_seen = done)
            .unwrap();
        assert!(progress_seen > 0);
        assert!(outcome.path.exists());
        assert!(outcome.staged.verified);
        assert_eq!(std::fs::read(&outcome.path).unwrap(), ctx.artifact_bytes);
    }

    #[test]
    fn checksum_mismatch_rejects_package() {
        let mut ctx = context("0.2.0", 1, 1);
        ctx.service.check().unwrap();
        let artifact_path = artifact_dir(ctx._dir.path()).join("fat-test.apk");
        std::fs::write(&artifact_path, b"tampered package").unwrap();
        let error = ctx.service.download_latest(&mut |_, _| {}).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("checksum") || message.contains("incomplete"));
        assert!(!ctx
            .service
            .state()
            .staged
            .as_ref()
            .is_some_and(|staged| Path::new(&staged.path).exists()));
    }

    #[test]
    fn signature_is_required_when_a_key_is_configured() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("app-data");
        let db_path = data_dir.join("workout.db");
        Database::open(&db_path).unwrap();
        let artifact_bytes = b"signed package".to_vec();
        let artifact_path = artifact_dir(dir.path()).join("fat.apk");
        std::fs::write(&artifact_path, &artifact_bytes).unwrap();
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        let public = hex::encode(signing_key.verifying_key().to_bytes());
        let signature = hex::encode(signing_key.sign(&artifact_bytes).to_bytes());
        write_manifest(
            dir.path(),
            &data_dir,
            "0.2.0",
            1,
            1,
            &artifact_path,
            Some(signature),
            Some(public),
        );
        let mut service =
            UpdateService::with_version(&data_dir, &db_path, 1, Version::parse("0.1.0").unwrap())
                .unwrap();
        service.check().unwrap();
        service.download_latest(&mut |_, _| {}).unwrap();
    }

    #[test]
    fn unsigned_package_is_rejected_when_a_key_is_configured() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("app-data");
        let db_path = data_dir.join("workout.db");
        Database::open(&db_path).unwrap();
        let artifact_bytes = b"unsigned package".to_vec();
        let artifact_path = artifact_dir(dir.path()).join("fat.apk");
        std::fs::write(&artifact_path, &artifact_bytes).unwrap();
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        let public = hex::encode(signing_key.verifying_key().to_bytes());
        write_manifest(
            dir.path(),
            &data_dir,
            "0.2.0",
            1,
            1,
            &artifact_path,
            None,
            Some(public),
        );
        let mut service =
            UpdateService::with_version(&data_dir, &db_path, 1, Version::parse("0.1.0").unwrap())
                .unwrap();
        service.check().unwrap();
        let error = service.download_latest(&mut |_, _| {}).unwrap_err();
        assert!(error.to_string().contains("signature"));
    }

    #[test]
    fn incompatible_schema_blocks_download() {
        let mut ctx = context("0.2.0", 2, 3);
        let check = ctx.service.check().unwrap();
        assert_eq!(check.availability, UpdateAvailability::IncompatibleDatabase);
        let error = ctx.service.download_latest(&mut |_, _| {}).unwrap_err();
        assert!(error.to_string().contains("database schema"));
    }

    #[test]
    fn install_without_command_stages_and_backs_up_database() {
        let mut ctx = context("0.2.0", 1, 1);
        ctx.service.check().unwrap();
        ctx.service.download_latest(&mut |_, _| {}).unwrap();
        let outcome = ctx.service.install_staged(false).unwrap();
        assert!(!outcome.installer_launched);
        assert!(outcome.message.contains("staged"));
        let backup = outcome.backup_path.unwrap();
        assert!(Path::new(&backup).exists());
        crate::database::backup::validate_backup(Path::new(&backup)).unwrap();
        let _ = ctx.db_path;
    }

    #[test]
    fn tampering_after_download_is_detected_before_install() {
        let mut ctx = context("0.2.0", 1, 1);
        ctx.service.check().unwrap();
        let outcome = ctx.service.download_latest(&mut |_, _| {}).unwrap();
        std::fs::write(&outcome.path, b"tampered after download").unwrap();
        let error = ctx.service.install_staged(false).unwrap_err();
        assert!(error.to_string().contains("checksum"));
    }

    #[test]
    fn update_check_works_offline_without_crashing_core_data() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        let db_path = data_dir.join("workout.db");
        let db = Database::open(&db_path).unwrap();
        let mut service =
            UpdateService::with_version(&data_dir, &db_path, 1, Version::parse("0.1.0").unwrap())
                .unwrap();
        let _ = service.check();
        let count: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM exercises", [], |row| row.get(0))
            .unwrap();
        assert!(count > 0);
    }
}
