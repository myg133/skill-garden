//! CLI Token 加密/解密
//!
//! 用于 cli.setup 生成的 config.toml 中的 token 字段。
//! 使用 AES-256-GCM 加密原始 API key，防止明文泄露。
//!
//! 加密 token 格式：skc_<base64(nonce || ciphertext || tag)>
//! - nonce: 12 bytes
//! - ciphertext + tag: AES-256-GCM 输出

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

const NONCE_SIZE: usize = 12; // 96-bit
const CLI_TOKEN_PREFIX: &str = "skc_";

/// 加密 API key，返回 skc_ 前缀的 base64 token
pub fn encrypt_api_key(plain: &str, key: &[u8; 32]) -> Result<String, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("invalid encryption key: {}", e))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plain.as_bytes())
        .map_err(|e| format!("encryption failed: {}", e))?;

    // 组合 nonce + ciphertext（ciphertext 已含 tag）
    let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(format!("{}{}", CLI_TOKEN_PREFIX, BASE64.encode(&combined)))
}

/// 解密 CLI token，返回原始 API key
/// 如果 token 不以 skc_ 开头，返回 None（表示不是加密 token）
pub fn decrypt_api_key(token: &str, key: &[u8; 32]) -> Result<Option<String>, String> {
    if !token.starts_with(CLI_TOKEN_PREFIX) {
        return Ok(None);
    }

    let encoded = &token[CLI_TOKEN_PREFIX.len()..];
    let combined = BASE64
        .decode(encoded)
        .map_err(|e| format!("invalid base64: {}", e))?;

    if combined.len() < NONCE_SIZE + 16 {
        // 至少 nonce + 最小 tag(16)
        return Err("token too short".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("invalid encryption key: {}", e))?;

    let plain = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("decryption failed: {}", e))?;

    let s = String::from_utf8(plain).map_err(|e| format!("invalid utf-8: {}", e))?;
    Ok(Some(s))
}

/// 从环境变量加载加密密钥（32 字节 hex）
/// 环境变量：AION_HIVE_CLI_ENCRYPTION_KEY
pub fn load_encryption_key() -> Result<[u8; 32], String> {
    let hex_str = std::env::var("AION_HIVE_CLI_ENCRYPTION_KEY").map_err(|_| {
        "AION_HIVE_CLI_ENCRYPTION_KEY environment variable not set".to_string()
    })?;

    let bytes = hex::decode(&hex_str).map_err(|e| format!("invalid hex key: {}", e))?;

    if bytes.len() != 32 {
        return Err(format!(
            "AION_HIVE_CLI_ENCRYPTION_KEY must be 32 bytes (64 hex chars), got {} bytes",
            bytes.len()
        ));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        // deterministic key for testing
        key.copy_from_slice(&[0xABu8; 32]);
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plain = "sk_test1234567890abcdef";

        let encrypted = encrypt_api_key(plain, &key).unwrap();
        assert!(encrypted.starts_with("skc_"));
        assert_ne!(encrypted, format!("skc_{}", plain));

        let decrypted = decrypt_api_key(&encrypted, &key).unwrap();
        assert_eq!(decrypted, Some(plain.to_string()));
    }

    #[test]
    fn test_decrypt_non_cli_token_returns_none() {
        let key = test_key();
        let result = decrypt_api_key("sk_plain_token", &key).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key = test_key();
        let plain = "sk_test123";
        let encrypted = encrypt_api_key(plain, &key).unwrap();

        let mut wrong_key = [0u8; 32];
        wrong_key.copy_from_slice(&[0xCDu8; 32]);
        let result = decrypt_api_key(&encrypted, &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_different_nonce_each_time() {
        let key = test_key();
        let plain = "sk_test";
        let e1 = encrypt_api_key(plain, &key).unwrap();
        let e2 = encrypt_api_key(plain, &key).unwrap();
        assert_ne!(e1, e2);
    }
}
