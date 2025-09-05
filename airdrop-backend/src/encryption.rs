use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_kms::{Client, types::DataKeySpec};
use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

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

    pub async fn decrypt_private_key(&self, encrypted_key: &str) -> Result<String> {
        let envelope: EnvelopeEncryption = serde_json::from_str(&String::from_utf8(
            base64::decode(encrypted_key)?
        )?)?;

        let decrypted = self.decrypt(&envelope).await?;
        Ok(String::from_utf8(decrypted)?)
    }
}
