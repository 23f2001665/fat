use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::models::update::UpdateConfig;
use crate::updates::release_check::validate_source_url;

pub const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024; // 1 GiB

pub struct DownloadResult {
    pub partial_path: std::path::PathBuf,
    pub sha256: String,
    pub bytes: u64,
}

/// Streams an update artifact into a `.part` file while hashing it.
///
/// The caller must verify the returned checksum/signature before renaming the
/// partial file to its final name. Corrupt or oversized downloads are removed.
pub fn download_package(
    config: &UpdateConfig,
    url: &str,
    partial_path: &Path,
    expected_size: Option<u64>,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> AppResult<DownloadResult> {
    validate_source_url(url, config.allow_file_urls)?;
    if let Some(parent) = partial_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(partial_path);

    let result = (|| -> AppResult<DownloadResult> {
        let mut output = File::create(partial_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut downloaded = 0u64;

        if url.starts_with("file://") {
            let source_path = Path::new(
                url.strip_prefix("file://")
                    .ok_or_else(|| AppError::update("Invalid local package URL"))?,
            );
            let mut input = File::open(source_path)?;
            let total = input.metadata().ok().map(|metadata| metadata.len());
            if let Some(size) = total {
                if size > MAX_PACKAGE_BYTES {
                    return Err(AppError::update("Update package is too large"));
                }
            }
            loop {
                let read = input.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                downloaded += read as u64;
                if downloaded > MAX_PACKAGE_BYTES {
                    return Err(AppError::update("Update package exceeds the size limit"));
                }
                hasher.update(&buffer[..read]);
                output.write_all(&buffer[..read])?;
                progress(downloaded, total.or(expected_size));
            }
        } else {
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(10))
                .timeout(Duration::from_secs(60))
                .build();
            let response = agent
                .get(url)
                .call()
                .map_err(|err| AppError::network(format!("Package download failed: {err}")))?;
            let total = response
                .header("Content-Length")
                .and_then(|value| value.parse::<u64>().ok())
                .or(expected_size);
            if total.is_some_and(|size| size > MAX_PACKAGE_BYTES) {
                return Err(AppError::update("Update package is too large"));
            }
            let mut input = response.into_reader();
            loop {
                let read = input.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                downloaded += read as u64;
                if downloaded > MAX_PACKAGE_BYTES {
                    return Err(AppError::update("Update package exceeds the size limit"));
                }
                hasher.update(&buffer[..read]);
                output.write_all(&buffer[..read])?;
                progress(downloaded, total);
            }
        }

        output.flush()?;
        output.sync_all()?;
        let sha256 = hex::encode(hasher.finalize());
        if let Some(expected) = expected_size {
            if downloaded != expected {
                return Err(AppError::update(format!(
                    "Package download is incomplete: expected {expected} bytes, received {downloaded}"
                )));
            }
        }
        Ok(DownloadResult {
            partial_path: partial_path.to_path_buf(),
            sha256,
            bytes: downloaded,
        })
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(partial_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updates::verification::sha256_bytes;

    fn config() -> UpdateConfig {
        UpdateConfig {
            allow_file_urls: true,
            ..Default::default()
        }
    }

    #[test]
    fn file_download_streams_and_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("package.bin");
        std::fs::write(&source, b"workout package").unwrap();
        let partial = dir.path().join("download.part");
        let url = format!("file://{}", source.display());
        let mut observed = 0u64;
        let result = download_package(&config(), &url, &partial, Some(15), &mut |done, _| {
            observed = done;
        })
        .unwrap();
        assert_eq!(result.bytes, 15);
        assert_eq!(observed, 15);
        assert_eq!(result.sha256, sha256_bytes(b"workout package"));
        assert!(partial.exists());
    }

    #[test]
    fn failed_source_does_not_leave_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let partial = dir.path().join("missing.part");
        let url = format!("file://{}/missing.bin", dir.path().display());
        assert!(download_package(&config(), &url, &partial, None, &mut |_, _| {}).is_err());
        assert!(!partial.exists());
    }
}
