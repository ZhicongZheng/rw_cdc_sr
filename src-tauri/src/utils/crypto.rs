use crate::utils::error::{AppError, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use once_cell::sync::Lazy;
use rand::Rng;

// 在生产环境中，这个密钥应该从环境变量或配置文件中读取
static ENCRYPTION_KEY: Lazy<[u8; 32]> = Lazy::new(|| {
    // TODO: 从环境变量或配置文件读取
    // 这里使用固定密钥仅用于开发，生产环境必须使用安全的密钥管理
    let key_str = std::env::var("ENCRYPTION_KEY")
        .unwrap_or_else(|_| "rw_cdc_sr_default_key_32_bytes!".to_string());
    let mut key = [0u8; 32];
    let key_bytes = key_str.as_bytes();
    let len = key_bytes.len().min(32);
    key[..len].copy_from_slice(&key_bytes[..len]);
    key
});

/// 加密字符串
pub fn encrypt(plaintext: &str) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(&*ENCRYPTION_KEY)
        .map_err(|e| AppError::Encryption(format!("Failed to create cipher: {}", e)))?;

    // 生成随机 nonce (12 bytes for AES-GCM)
    let mut rng = rand::thread_rng();
    let nonce_bytes: [u8; 12] = rng.r#gen(); // gen 是 Rust 2024 的保留关键字，需要转义
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 加密
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Encryption(format!("Encryption failed: {}", e)))?;

    // 将 nonce 和 ciphertext 组合并编码为 base64
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(&result))
}

/// 解密字符串
pub fn decrypt(encrypted: &str) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(&*ENCRYPTION_KEY)
        .map_err(|e| AppError::Encryption(format!("Failed to create cipher: {}", e)))?;

    // 解码 base64
    let encrypted_data = general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|e| AppError::Encryption(format!("Base64 decode failed: {}", e)))?;

    if encrypted_data.len() < 12 {
        return Err(AppError::Encryption(
            "Invalid encrypted data: too short".to_string(),
        ));
    }

    // 分离 nonce 和 ciphertext
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // 解密
    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::Encryption(format!("Decryption failed: {}", e)))?;

    String::from_utf8(plaintext_bytes)
        .map_err(|e| AppError::Encryption(format!("UTF-8 decode failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let original = "my_secret_password";
        let encrypted = encrypt(original).unwrap();
        assert_ne!(encrypted, original);

        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let result = decrypt("invalid_base64!");
        assert!(result.is_err());
    }
}
