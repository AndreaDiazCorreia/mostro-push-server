use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use log::{debug, error};
use secp256k1::{PublicKey, SecretKey, Secp256k1};
use sha2::Sha256;

const HKDF_SALT: &[u8] = b"mostro-push-v1";
const HKDF_INFO: &[u8] = b"mostro-token-encryption";

const PLATFORM_ANDROID: u8 = 0x02;
const PLATFORM_IOS: u8 = 0x01;

const PADDED_PAYLOAD_SIZE: usize = 220;
const EPHEMERAL_PUBKEY_SIZE: usize = 33;
const NONCE_SIZE: usize = 12;
const AUTH_TAG_SIZE: usize = 16;
pub const ENCRYPTED_TOKEN_SIZE: usize = EPHEMERAL_PUBKEY_SIZE + NONCE_SIZE + PADDED_PAYLOAD_SIZE + AUTH_TAG_SIZE;

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Android,
    Ios,
}

impl Platform {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            PLATFORM_ANDROID => Some(Platform::Android),
            PLATFORM_IOS => Some(Platform::Ios),
            _ => None,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            Platform::Android => PLATFORM_ANDROID,
            Platform::Ios => PLATFORM_IOS,
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Android => write!(f, "android"),
            Platform::Ios => write!(f, "ios"),
        }
    }
}

#[derive(Debug)]
pub struct DecryptedToken {
    pub platform: Platform,
    pub device_token: String,
}

pub struct TokenCrypto {
    secret_key: SecretKey,
    public_key: PublicKey,
    secp: Secp256k1<secp256k1::All>,
}

impl TokenCrypto {
    pub fn new(secret_key_hex: &str) -> Result<Self, CryptoError> {
        let secp = Secp256k1::new();
        
        let secret_key_bytes = hex::decode(secret_key_hex)
            .map_err(|_| CryptoError::InvalidSecretKey)?;
        
        let secret_key = SecretKey::from_slice(&secret_key_bytes)
            .map_err(|_| CryptoError::InvalidSecretKey)?;
        
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        
        Ok(Self {
            secret_key,
            public_key,
            secp,
        })
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key.serialize())
    }

    pub fn decrypt_token(&self, encrypted_token: &[u8]) -> Result<DecryptedToken, CryptoError> {
        if encrypted_token.len() != ENCRYPTED_TOKEN_SIZE {
            error!(
                "Invalid token size: expected {}, got {}",
                ENCRYPTED_TOKEN_SIZE,
                encrypted_token.len()
            );
            return Err(CryptoError::InvalidTokenSize);
        }

        // Extract components
        let ephemeral_pubkey_bytes = &encrypted_token[0..EPHEMERAL_PUBKEY_SIZE];
        let nonce_bytes = &encrypted_token[EPHEMERAL_PUBKEY_SIZE..EPHEMERAL_PUBKEY_SIZE + NONCE_SIZE];
        let ciphertext = &encrypted_token[EPHEMERAL_PUBKEY_SIZE + NONCE_SIZE..];

        debug!("Ephemeral pubkey: {}", hex::encode(ephemeral_pubkey_bytes));
        debug!("Nonce: {}", hex::encode(nonce_bytes));
        debug!("Ciphertext length: {}", ciphertext.len());

        // Parse ephemeral public key
        let ephemeral_pubkey = PublicKey::from_slice(ephemeral_pubkey_bytes)
            .map_err(|e| {
                error!("Failed to parse ephemeral pubkey: {}", e);
                CryptoError::InvalidEphemeralKey
            })?;

        // Derive shared secret via ECDH
        let shared_point = secp256k1::ecdh::SharedSecret::new(&ephemeral_pubkey, &self.secret_key);
        let shared_x = shared_point.secret_bytes();

        // Derive encryption key using HKDF
        let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), &shared_x);
        let mut encryption_key = [0u8; 32];
        hk.expand(HKDF_INFO, &mut encryption_key)
            .map_err(|_| CryptoError::HkdfError)?;

        // Decrypt with ChaCha20-Poly1305
        let cipher = ChaCha20Poly1305::new_from_slice(&encryption_key)
            .map_err(|_| CryptoError::CipherError)?;
        let nonce = Nonce::from_slice(nonce_bytes);

        let padded_payload = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| {
                error!("Decryption failed: {}", e);
                CryptoError::DecryptionFailed
            })?;

        if padded_payload.len() != PADDED_PAYLOAD_SIZE {
            error!(
                "Invalid payload size after decryption: expected {}, got {}",
                PADDED_PAYLOAD_SIZE,
                padded_payload.len()
            );
            return Err(CryptoError::InvalidPayloadSize);
        }

        // Parse padded payload
        let platform_byte = padded_payload[0];
        let token_length = u16::from_be_bytes([padded_payload[1], padded_payload[2]]) as usize;

        if token_length > PADDED_PAYLOAD_SIZE - 3 {
            error!("Token length {} exceeds maximum", token_length);
            return Err(CryptoError::InvalidTokenLength);
        }

        let platform = Platform::from_byte(platform_byte)
            .ok_or(CryptoError::InvalidPlatform)?;

        let device_token_bytes = &padded_payload[3..3 + token_length];
        let device_token = String::from_utf8(device_token_bytes.to_vec())
            .map_err(|_| CryptoError::InvalidTokenEncoding)?;

        debug!("Decrypted token for platform {:?}, length {}", platform, token_length);

        Ok(DecryptedToken {
            platform,
            device_token,
        })
    }
}

#[derive(Debug)]
pub enum CryptoError {
    InvalidSecretKey,
    InvalidTokenSize,
    InvalidEphemeralKey,
    HkdfError,
    CipherError,
    DecryptionFailed,
    InvalidPayloadSize,
    InvalidTokenLength,
    InvalidPlatform,
    InvalidTokenEncoding,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::InvalidSecretKey => write!(f, "Invalid secret key"),
            CryptoError::InvalidTokenSize => write!(f, "Invalid encrypted token size"),
            CryptoError::InvalidEphemeralKey => write!(f, "Invalid ephemeral public key"),
            CryptoError::HkdfError => write!(f, "HKDF derivation failed"),
            CryptoError::CipherError => write!(f, "Cipher initialization failed"),
            CryptoError::DecryptionFailed => write!(f, "Decryption failed"),
            CryptoError::InvalidPayloadSize => write!(f, "Invalid payload size after decryption"),
            CryptoError::InvalidTokenLength => write!(f, "Invalid token length in payload"),
            CryptoError::InvalidPlatform => write!(f, "Invalid platform identifier"),
            CryptoError::InvalidTokenEncoding => write!(f, "Invalid token encoding"),
        }
    }
}

impl std::error::Error for CryptoError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn create_test_encrypted_token(
        server_pubkey: &PublicKey,
        platform: Platform,
        device_token: &str,
    ) -> Vec<u8> {
        let secp = Secp256k1::new();
        
        // Generate ephemeral keypair
        let mut rng = rand::thread_rng();
        let ephemeral_secret = SecretKey::new(&mut rng);
        let ephemeral_pubkey = PublicKey::from_secret_key(&secp, &ephemeral_secret);

        // Derive shared secret
        let shared_point = secp256k1::ecdh::SharedSecret::new(server_pubkey, &ephemeral_secret);
        let shared_x = shared_point.secret_bytes();

        // Derive encryption key
        let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), &shared_x);
        let mut encryption_key = [0u8; 32];
        hk.expand(HKDF_INFO, &mut encryption_key).unwrap();

        // Create padded payload
        let token_bytes = device_token.as_bytes();
        let mut padded_payload = vec![0u8; PADDED_PAYLOAD_SIZE];
        padded_payload[0] = platform.to_byte();
        padded_payload[1..3].copy_from_slice(&(token_bytes.len() as u16).to_be_bytes());
        padded_payload[3..3 + token_bytes.len()].copy_from_slice(token_bytes);
        
        // Fill rest with random padding
        rng.fill_bytes(&mut padded_payload[3 + token_bytes.len()..]);

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let cipher = ChaCha20Poly1305::new_from_slice(&encryption_key).unwrap();
        let ciphertext = cipher.encrypt(nonce, padded_payload.as_slice()).unwrap();

        // Combine: ephemeral_pubkey || nonce || ciphertext
        let mut encrypted_token = Vec::with_capacity(ENCRYPTED_TOKEN_SIZE);
        encrypted_token.extend_from_slice(&ephemeral_pubkey.serialize());
        encrypted_token.extend_from_slice(&nonce_bytes);
        encrypted_token.extend_from_slice(&ciphertext);

        encrypted_token
    }

    #[test]
    fn test_decrypt_token() {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let server_secret = SecretKey::new(&mut rng);
        let server_pubkey = PublicKey::from_secret_key(&secp, &server_secret);

        let crypto = TokenCrypto::new(&hex::encode(server_secret.secret_bytes())).unwrap();

        let device_token = "test_fcm_token_12345";
        let encrypted = create_test_encrypted_token(&server_pubkey, Platform::Android, device_token);

        let decrypted = crypto.decrypt_token(&encrypted).unwrap();
        assert_eq!(decrypted.platform, Platform::Android);
        assert_eq!(decrypted.device_token, device_token);
    }
}
