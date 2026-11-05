use std::fs::File;
use std::io::Read;
use std::path::Path;

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::models::update::ReleaseArtifact;

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

pub fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Hexadecimal comparison whose running time does not depend on the first
/// differing character. Case-insensitive because manifests may use uppercase.
pub fn checksum_matches(expected: &str, actual: &str) -> bool {
    let expected = expected.trim().to_ascii_lowercase();
    let actual = actual.trim().to_ascii_lowercase();
    if expected.len() != actual.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in expected.bytes().zip(actual.bytes()) {
        difference |= left ^ right;
    }
    difference == 0
}

pub fn validate_sha256(value: &str) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.len() != 64 || !trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::update(
            "Release metadata contains an invalid SHA-256 checksum",
        ));
    }
    Ok(())
}

pub fn parse_public_key(value: &str) -> AppResult<VerifyingKey> {
    let bytes = hex::decode(value.trim())
        .map_err(|_| AppError::update("Publisher public key is not valid hexadecimal"))?;
    let key_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| AppError::update("Publisher public key must be 32 bytes (64 hex chars)"))?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| AppError::update("Publisher public key is not a valid Ed25519 key"))
}

pub fn verify_signature(bytes: &[u8], signature_hex: &str, public_key_hex: &str) -> AppResult<()> {
    let key = parse_public_key(public_key_hex)?;
    let signature_bytes = hex::decode(signature_hex.trim())
        .map_err(|_| AppError::update("Package signature is not valid hexadecimal"))?;
    let signature_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| AppError::update("Package signature must be 64 bytes (128 hex chars)"))?;
    let signature = Signature::from_bytes(&signature_bytes);
    key.verify_strict(bytes, &signature)
        .map_err(|_| AppError::update("Package signature verification failed"))
}

/// Mandatory checksum verification and optional-but-preferred publisher signature.
///
/// When a public key is configured in the update configuration, a signature is
/// required: a package with only a checksum is rejected. This prevents a
/// compromised release server from substituting both the artifact and hash.
pub fn verify_package(
    path: &Path,
    artifact: &ReleaseArtifact,
    public_key_hex: Option<&str>,
) -> AppResult<String> {
    validate_sha256(&artifact.sha256)?;
    let actual = sha256_file(path)?;
    if !checksum_matches(&artifact.sha256, &actual) {
        return Err(AppError::update(
            "Package checksum does not match the trusted release metadata",
        ));
    }

    if let Some(key) = public_key_hex {
        let signature = artifact.signature.as_deref().ok_or_else(|| {
            AppError::update("Publisher signing key is configured but the release has no signature")
        })?;
        let bytes = std::fs::read(path)?;
        verify_signature(&bytes, signature, key)?;
    }

    if let Some(size) = artifact.size_bytes {
        let actual_size = std::fs::metadata(path)?.len();
        if actual_size != size {
            return Err(AppError::update(format!(
                "Package size mismatch: expected {size} bytes, downloaded {actual_size}"
            )));
        }
    }
    Ok(actual)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_comparison_is_case_insensitive() {
        let digest = sha256_bytes(b"workout");
        assert!(checksum_matches(&digest.to_uppercase(), &digest));
        assert!(!checksum_matches(&digest, &sha256_bytes(b"other")));
    }

    #[test]
    fn invalid_checksums_are_rejected() {
        assert!(validate_sha256("abc").is_err());
        assert!(validate_sha256(&"g".repeat(64)).is_err());
        assert!(validate_sha256(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn ed25519_signatures_verify_and_tampering_fails() {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public = hex::encode(signing_key.verifying_key().to_bytes());
        let data = b"verified release package";
        let signature = hex::encode(signing_key.sign(data).to_bytes());
        verify_signature(data, &signature, &public).unwrap();
        assert!(verify_signature(b"tampered", &signature, &public).is_err());
    }
}
