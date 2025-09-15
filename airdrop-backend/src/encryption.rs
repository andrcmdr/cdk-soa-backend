use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_kms::{Client, types::DataKeySpec};
use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use secp256k1::{Secp256k1, SecretKey};
use alloy_primitives::hex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeEncryption {
    pub encrypted_data: Vec<u8>,
    pub encrypted_data_key: Vec<u8>,
    pub nonce: Vec<u8>,
}

pub struct KmsEnvelopeEncryption {
    kms_client: Client,
    kms_key_id: String,
}

impl KmsEnvelopeEncryption {
    pub async fn new(region: &str, kms_key_id: String) -> Result<Self> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let kms_client = Client::new(&config);

        Ok(Self {
            kms_client,
            kms_key_id,
        })
    }

    pub async fn encrypt(&self, plaintext: &[u8]) -> Result<EnvelopeEncryption> {
        // Generate data key from KMS
        let data_key_response = self.kms_client
            .generate_data_key()
            .key_id(&self.kms_key_id)
            .key_spec(DataKeySpec::Aes256)
            .send()
            .await?;

        let plaintext_data_key = data_key_response.plaintext()
            .ok_or_else(|| anyhow::anyhow!("No plaintext data key returned"))?;
        let encrypted_data_key = data_key_response.ciphertext_blob()
            .ok_or_else(|| anyhow::anyhow!("No encrypted data key returned"))?;

        // Encrypt data with the data key using AES-GCM
        let cipher = Aes256Gcm::new(GenericArray::from_slice(plaintext_data_key.as_ref()));

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted_data = cipher.encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        Ok(EnvelopeEncryption {
            encrypted_data,
            encrypted_data_key: encrypted_data_key.as_ref().to_vec(),
            nonce: nonce_bytes.to_vec(),
        })
    }

    pub async fn decrypt(&self, envelope: &EnvelopeEncryption) -> Result<Vec<u8>> {
        // Decrypt the data key using KMS
        let decrypt_response = self.kms_client
            .decrypt()
            .ciphertext_blob(aws_smithy_types::Blob::new(envelope.encrypted_data_key.clone()))
            .send()
            .await?;

        let plaintext_data_key = decrypt_response.plaintext()
            .ok_or_else(|| anyhow::anyhow!("Failed to decrypt data key"))?;

        // Decrypt data using the decrypted data key
        let cipher = Aes256Gcm::new(GenericArray::from_slice(plaintext_data_key.as_ref()));
        let nonce = Nonce::from_slice(&envelope.nonce);

        let plaintext = cipher.decrypt(nonce, envelope.encrypted_data.as_ref())
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        Ok(plaintext)
    }

    pub async fn generate_and_encrypt_private_key(&self) -> Result<String> {
        tracing::info!("Generating new private key");

        // Generate a new secp256k1 private key
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let secret_key = SecretKey::new(&mut rng);

        // Convert to hex string (without 0x prefix)
        let private_key_hex = hex::encode(secret_key.secret_bytes());

        tracing::info!("Encrypting private key with KMS envelope encryption");

        // Encrypt the private key
        let envelope = self.encrypt(private_key_hex.as_bytes()).await?;

        // Serialize and encode as base64
        let serialized = serde_json::to_string(&envelope)?;
        let encoded = base64::encode(serialized.as_bytes());

        tracing::info!("Private key generated and encrypted successfully");

        Ok(encoded)
    }

    pub async fn decrypt_private_key(&self, encrypted_key: &str) -> Result<String> {
        if encrypted_key.is_empty() {
            return Err(anyhow::anyhow!("Encrypted private key is empty"));
        }

        let decoded = base64::decode(encrypted_key)?;
        let envelope: EnvelopeEncryption = serde_json::from_str(&String::from_utf8(decoded)?)?;

        let decrypted = self.decrypt(&envelope).await?;
        let private_key = String::from_utf8(decrypted)?;

        // Validate the private key format (should be 64 hex characters)
        if private_key.len() != 64 {
            return Err(anyhow::anyhow!("Invalid private key length"));
        }

        // Validate hex format
        hex::decode(&private_key)
            .map_err(|_| anyhow::anyhow!("Invalid private key format"))?;

        Ok(format!("0x{}", private_key))
    }

    pub async fn get_or_create_private_key(&self, encrypted_key: &str) -> Result<String> {
        if encrypted_key.is_empty() {
            tracing::info!("No encrypted private key found, generating new one");
            return self.generate_and_encrypt_private_key().await;
        }

        tracing::info!("Decrypting existing private key");
        self.decrypt_private_key(encrypted_key).await
    }
}
