use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

const ENVELOPE_VERSION: u8 = 1;
const ENVELOPE_ALGORITHM: &str = "AES-256-GCM";
const KEY_BYTES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretCryptoError {
    pub code: String,
    pub message: String,
}

pub trait SecretKeyProvider {
    fn get_or_create_key(&self) -> Result<[u8; KEY_BYTES], SecretCryptoError>;
}

#[derive(Clone, Debug)]
pub struct InMemorySecretKeyProvider {
    key: [u8; KEY_BYTES],
}

impl InMemorySecretKeyProvider {
    pub fn new(key: [u8; KEY_BYTES]) -> Self {
        Self { key }
    }
}

impl SecretKeyProvider for InMemorySecretKeyProvider {
    fn get_or_create_key(&self) -> Result<[u8; KEY_BYTES], SecretCryptoError> {
        Ok(self.key)
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug, Default)]
pub struct LinuxSecretServiceKeyProvider;

#[cfg(target_os = "linux")]
impl SecretKeyProvider for LinuxSecretServiceKeyProvider {
    fn get_or_create_key(&self) -> Result<[u8; KEY_BYTES], SecretCryptoError> {
        let entry = keyring::Entry::new("com.snackbit.daydream", "settings-encryption-key")
            .map_err(secret_service_error)?;

        match entry.get_secret() {
            Ok(secret) => decode_key(&secret),
            Err(keyring::Error::NoEntry) => {
                let key = Aes256Gcm::generate_key(&mut OsRng);
                entry
                    .set_secret(key.as_slice())
                    .map_err(secret_service_error)?;
                decode_key(key.as_slice())
            }
            Err(error) => Err(secret_service_error(error)),
        }
    }
}

#[cfg(not(target_os = "linux"))]
#[derive(Clone, Debug, Default)]
pub struct PlatformSecretKeyProvider;

#[cfg(not(target_os = "linux"))]
impl SecretKeyProvider for PlatformSecretKeyProvider {
    fn get_or_create_key(&self) -> Result<[u8; KEY_BYTES], SecretCryptoError> {
        Err(SecretCryptoError {
            code: "secret_crypto_not_implemented".to_string(),
            message: "Encrypted settings are not implemented on this platform yet.".to_string(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedSettingEnvelope {
    pub alg: String,
    pub ciphertext: String,
    pub nonce: String,
    pub v: u8,
}

pub fn encrypt_sensitive_setting(
    plaintext: &str,
    provider: &impl SecretKeyProvider,
) -> Result<String, SecretCryptoError> {
    let key = provider.get_or_create_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(crypto_error)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(crypto_error)?;

    serde_json::to_string(&EncryptedSettingEnvelope {
        alg: ENVELOPE_ALGORITHM.to_string(),
        ciphertext: STANDARD.encode(ciphertext),
        nonce: STANDARD.encode(nonce),
        v: ENVELOPE_VERSION,
    })
    .map_err(crypto_error)
}

#[cfg(target_os = "linux")]
pub fn encrypt_sensitive_setting_with_platform_key(
    plaintext: &str,
) -> Result<String, SecretCryptoError> {
    encrypt_sensitive_setting(plaintext, &LinuxSecretServiceKeyProvider)
}

#[cfg(not(target_os = "linux"))]
pub fn encrypt_sensitive_setting_with_platform_key(
    plaintext: &str,
) -> Result<String, SecretCryptoError> {
    encrypt_sensitive_setting(plaintext, &PlatformSecretKeyProvider)
}

pub fn decrypt_sensitive_setting(
    envelope_json: &str,
    provider: &impl SecretKeyProvider,
) -> Result<String, SecretCryptoError> {
    let envelope: EncryptedSettingEnvelope =
        serde_json::from_str(envelope_json).map_err(crypto_error)?;
    if envelope.v != ENVELOPE_VERSION || envelope.alg != ENVELOPE_ALGORITHM {
        return Err(SecretCryptoError {
            code: "secret_crypto_unsupported_envelope".to_string(),
            message: "Encrypted setting envelope is not supported.".to_string(),
        });
    }

    let key = provider.get_or_create_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(crypto_error)?;
    let nonce = STANDARD.decode(envelope.nonce).map_err(crypto_error)?;
    let ciphertext = STANDARD.decode(envelope.ciphertext).map_err(crypto_error)?;
    let plaintext = cipher
        .decrypt(nonce.as_slice().into(), ciphertext.as_ref())
        .map_err(crypto_error)?;

    String::from_utf8(plaintext).map_err(crypto_error)
}

#[cfg(target_os = "linux")]
pub fn decrypt_sensitive_setting_with_platform_key(
    envelope_json: &str,
) -> Result<String, SecretCryptoError> {
    decrypt_sensitive_setting(envelope_json, &LinuxSecretServiceKeyProvider)
}

#[cfg(not(target_os = "linux"))]
pub fn decrypt_sensitive_setting_with_platform_key(
    envelope_json: &str,
) -> Result<String, SecretCryptoError> {
    decrypt_sensitive_setting(envelope_json, &PlatformSecretKeyProvider)
}

#[cfg(target_os = "linux")]
fn secret_service_error(error: impl std::fmt::Display) -> SecretCryptoError {
    SecretCryptoError {
        code: "secret_service_unavailable".to_string(),
        message: format!("Linux Secret Service failed: {error}"),
    }
}

fn decode_key(secret: &[u8]) -> Result<[u8; KEY_BYTES], SecretCryptoError> {
    secret.try_into().map_err(|_error| SecretCryptoError {
        code: "secret_crypto_invalid_key".to_string(),
        message: "Stored settings encryption key has invalid length.".to_string(),
    })
}

fn crypto_error(error: impl std::fmt::Display) -> SecretCryptoError {
    SecretCryptoError {
        code: "secret_crypto_failed".to_string(),
        message: format!("Encrypted setting operation failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trips_without_plaintext() {
        let provider = InMemorySecretKeyProvider::new([7; KEY_BYTES]);
        let envelope = encrypt_sensitive_setting("sk-private", &provider).expect("encrypt");

        assert!(!envelope.contains("sk-private"));
        assert!(envelope.contains(ENVELOPE_ALGORITHM));

        let plaintext = decrypt_sensitive_setting(&envelope, &provider).expect("decrypt");

        assert_eq!(plaintext, "sk-private");
    }
}
