//! One-time migration for vaults created by the old encrypted version.
//!
//! Mahzen used to encrypt every note and attachment with AES-256-GCM under a
//! master password. That layer has been removed, but existing vaults still
//! contain `ENCD1`-framed files. This module decrypts them **once**, in place,
//! after the user supplies their old master password, and then the vault is an
//! ordinary folder of Markdown files.
//!
//! It is deliberately narrow: it can only *read* the legacy format, never
//! write it, and it is not used by the app after migration.

use std::path::{Path, PathBuf};

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::Argon2;
use serde::Deserialize;

const MAGIC: &[u8; 5] = b"ENCD1";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const VERIFIER_PLAIN: &[u8] = b"encrypted-note::password-verifier::v1";

/// The legacy keyfile: Argon2id salt plus a password verifier.
#[derive(Deserialize)]
pub struct LegacyKeyfile {
    pub salt: String,
    pub verifier_nonce: String,
    pub verifier_ct: String,
}

impl LegacyKeyfile {
    pub fn path() -> PathBuf {
        crate::config::dir().join("keyfile.json")
    }

    fn load() -> Option<Self> {
        let data = std::fs::read_to_string(Self::path()).ok()?;
        serde_json::from_str(&data).ok()
    }
}

fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .expect("Argon2 key derivation cannot fail for a 32-byte output");
    key
}

/// Whether a file starts with the legacy `ENCD1` frame.
pub fn is_encrypted_file(path: &Path) -> bool {
    let Ok(data) = std::fs::read(path) else {
        return false;
    };
    data.len() > MAGIC.len() && &data[..MAGIC.len()] == MAGIC
}

/// Whether the vault folder still contains encrypted notes or attachments.
pub fn vault_needs_migration(root: &Path) -> bool {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if is_encrypted_file(&path) {
                return true;
            }
        }
    }
    false
}

/// Verify `password` against the legacy keyfile and derive the vault key.
pub fn unlock(password: &str) -> Result<[u8; KEY_LEN], String> {
    let keyfile = LegacyKeyfile::load().ok_or_else(|| {
        "The old keyfile is missing, so these notes cannot be decrypted.".to_string()
    })?;
    let salt = hex_decode(&keyfile.salt).ok_or("Corrupted keyfile (salt).")?;
    let nonce = hex_decode(&keyfile.verifier_nonce).ok_or("Corrupted keyfile (nonce).")?;
    let ct = hex_decode(&keyfile.verifier_ct).ok_or("Corrupted keyfile (verifier).")?;
    let key = derive_key(password, &salt);
    if nonce.len() != NONCE_LEN {
        return Err("Corrupted keyfile (nonce length).".into());
    }
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    match cipher.decrypt(Nonce::from_slice(&nonce), ct.as_slice()) {
        Ok(plain) if plain.as_slice() == VERIFIER_PLAIN => Ok(key),
        _ => Err("Incorrect password.".into()),
    }
}

fn decrypt_bytes(key: &[u8; KEY_LEN], data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < MAGIC.len() + NONCE_LEN || &data[..MAGIC.len()] != MAGIC {
        return None;
    }
    let nonce = &data[MAGIC.len()..MAGIC.len() + NONCE_LEN];
    let ciphertext = &data[MAGIC.len() + NONCE_LEN..];
    let cipher = Aes256Gcm::new_from_slice(key).ok()?;
    cipher.decrypt(Nonce::from_slice(nonce), ciphertext).ok()
}

/// What a migration run did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub decrypted: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// Decrypt every `ENCD1` file under `root` in place. The keyfile is removed on
/// success, so the migration cannot silently run twice with a stale password.
pub fn migrate_vault(root: &Path, key: &[u8; KEY_LEN]) -> Report {
    let mut report = Report::default();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(data) = std::fs::read(&path) else {
                continue;
            };
            if data.len() <= MAGIC.len() || &data[..MAGIC.len()] != MAGIC {
                continue;
            }
            match decrypt_bytes(key, &data) {
                Some(plain) => match crate::vault::Vault::write_bytes(&path, &plain) {
                    Ok(()) => report.decrypted += 1,
                    Err(e) => {
                        report.failed += 1;
                        report.errors.push(format!("{}: {e}", path.display()));
                    }
                },
                None => {
                    report.failed += 1;
                    report
                        .errors
                        .push(format!("{}: could not decrypt", path.display()));
                }
            }
        }
    }
    if report.failed == 0 {
        let _ = std::fs::remove_file(LegacyKeyfile::path());
    }
    report
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(bytes[i + 1])?;
        out.push(hi * 16 + lo);
        i += 2;
    }
    Some(out)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip_and_rejects_odd_input() {
        assert_eq!(hex_decode("00ff10"), Some(vec![0, 255, 16]));
        assert_eq!(hex_decode("abc"), None);
        assert_eq!(hex_decode("zz"), None);
    }

    #[test]
    fn plain_text_is_not_flagged_as_encrypted() {
        let dir = std::env::temp_dir().join(format!("mahzen-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let plain = dir.join("a.md");
        std::fs::write(&plain, b"# hello").unwrap();
        assert!(!is_encrypted_file(&plain));
        let enc = dir.join("b.md");
        let mut framed = MAGIC.to_vec();
        framed.extend_from_slice(&[0u8; NONCE_LEN + 8]);
        std::fs::write(&enc, &framed).unwrap();
        assert!(is_encrypted_file(&enc));
        assert!(vault_needs_migration(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
