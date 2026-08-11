//! P0 field/blob encryption using XChaCha20-Poly1305 (libsodium-equivalent AEAD).

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::path::Path;

const NONCE_LEN: usize = 24;

pub struct MasterKey {
    key: [u8; 32],
}

impl MasterKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { key: bytes }
    }

    pub fn load_or_create(data_dir: &Path) -> std::io::Result<(Self, Option<String>)> {
        crate::paths::ensure_dir(data_dir)?;
        let key_path = data_dir.join("master.key");
        if key_path.exists() {
            let hex_str = std::fs::read_to_string(&key_path)?;
            let bytes = hex::decode(hex_str.trim())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if bytes.len() != 32 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "master.key must be 32 bytes hex",
                ));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok((Self { key }, None));
        }

        if let Ok(env) = std::env::var("OPENANTY_MASTER_KEY") {
            let digest = Sha256::digest(env.as_bytes());
            let mut key = [0u8; 32];
            key.copy_from_slice(&digest);
            return Ok((Self { key }, None));
        }

        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        std::fs::write(&key_path, hex::encode(key))?;

        // Recovery key: base32-ish hex for user backup
        let recovery = format!("OA-RECOVERY-{}", hex::encode(key));
        let recovery_path = data_dir.join("recovery.key.ONCE.txt");
        std::fs::write(
            &recovery_path,
            format!(
                "Open Anty recovery key — SAVE THIS OFFLINE, then delete this file.\n\n{recovery}\n"
            ),
        )?;

        Ok((Self { key }, Some(recovery)))
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key).map_err(|e| e.to_string())?;
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ct = cipher
            .encrypt(XNonce::from_slice(&nonce), plaintext)
            .map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    pub fn decrypt(&self, blob: &[u8]) -> Result<Vec<u8>, String> {
        if blob.len() < NONCE_LEN + 16 {
            return Err("ciphertext too short".into());
        }
        let (nonce, ct) = blob.split_at(NONCE_LEN);
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key).map_err(|e| e.to_string())?;
        cipher
            .decrypt(XNonce::from_slice(nonce), ct)
            .map_err(|_| "decrypt failed — wrong key or corrupt data".into())
    }

    pub fn encrypt_json<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>, String> {
        let plain = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        self.encrypt(&plain)
    }

    pub fn decrypt_json<T: serde::de::DeserializeOwned>(&self, blob: &[u8]) -> Result<T, String> {
        let plain = self.decrypt(blob)?;
        serde_json::from_slice(&plain).map_err(|e| e.to_string())
    }
}
