use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use hiero_did_core::error::DIDError;
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::vault_config::{
    VaultAuth,
    VaultSignerConfig,
};

pub struct VaultApi {
    client: Client,
    vault_url: String,
    mount_path: String,
    token: String,
}

#[derive(Deserialize)]
struct AuthResp {
    auth: AuthData,
}
#[derive(Deserialize)]
struct AuthData {
    client_token: String,
}

#[derive(Deserialize)]
struct SignResp {
    data: SignData,
}
#[derive(Deserialize)]
struct SignData {
    signature: String,
}

#[derive(Deserialize)]
struct KeyResp {
    data: KeyData,
}
#[derive(Deserialize)]
struct KeyData {
    keys: std::collections::HashMap<String, KeyVersion>,
}
#[derive(Deserialize)]
struct KeyVersion {
    public_key: String,
}

impl VaultApi {
    pub fn new(cfg: &VaultSignerConfig) -> Result<Self, DIDError> {
        let client = Client::new();
        let token = Self::login(&client, &cfg.vault_url, &cfg.auth)?;
        Ok(Self {
            client,
            vault_url: cfg.vault_url.trim_end_matches('/').to_string(),
            mount_path: cfg.mount_path.clone(),
            token,
        })
    }

    fn login(client: &Client, vault_url: &str, auth: &VaultAuth) -> Result<String, DIDError> {
        match auth {
            VaultAuth::Token(t) => Ok(t.clone()),
            VaultAuth::AppRole { role_id, secret_id } => {
                let url = format!("{}/v1/auth/approle/login", vault_url.trim_end_matches('/'));
                let body = serde_json::json!({
                    "role_id": role_id,
                    "secret_id": secret_id,
                });
                let resp: AuthResp = client
                    .post(&url)
                    .json(&body)
                    .send()
                    .map_err(|e| DIDError::InternalError(e.to_string()))?
                    .json()
                    .map_err(|e| DIDError::InternalError(e.to_string()))?;
                Ok(resp.auth.client_token)
            }
        }
    }

    pub fn fetch_public_key(&self, key_name: &str) -> Result<Vec<u8>, DIDError> {
        let url = format!("{}/v1/{}/keys/{}", self.vault_url, self.mount_path, key_name);
        let resp: KeyResp = self
            .client
            .get(&url)
            .header("X-Vault-Token", &self.token)
            .send()
            .map_err(|e| DIDError::InternalError(e.to_string()))?
            .json()
            .map_err(|e| DIDError::InternalError(e.to_string()))?;

        public_key_from_key_response(&resp, key_name)
    }

    pub fn sign_ed25519(&self, key_name: &str, message: &[u8]) -> Result<Vec<u8>, DIDError> {
        let url = format!("{}/v1/{}/sign/{}", self.vault_url, self.mount_path, key_name);
        let body = serde_json::json!({"input": B64.encode(message)});

        let resp: SignResp = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.token)
            .json(&body)
            .send()
            .map_err(|e| DIDError::InternalError(e.to_string()))?
            .json()
            .map_err(|e| DIDError::InternalError(e.to_string()))?;

        decode_vault_signature(&resp.data.signature)
    }
}

fn public_key_from_key_response(resp: &KeyResp, key_name: &str) -> Result<Vec<u8>, DIDError> {
    let ver = latest_key_version(&resp.data.keys)
        .ok_or_else(|| DIDError::NotFound(format!("Vault key not found: {key_name}")))?;

    B64.decode(&ver.public_key).map_err(|e| DIDError::InternalError(e.to_string()))
}

fn decode_vault_signature(signature: &str) -> Result<Vec<u8>, DIDError> {
    let raw_b64 = signature
        .strip_prefix("vault:")
        .and_then(|rest| rest.split_once(':').map(|(_, raw_b64)| raw_b64))
        .ok_or_else(|| DIDError::InternalError("unexpected vault signature format".into()))?;

    B64.decode(raw_b64).map_err(|e| DIDError::InternalError(e.to_string()))
}

fn latest_key_version(keys: &std::collections::HashMap<String, KeyVersion>) -> Option<&KeyVersion> {
    keys.iter()
        .filter_map(|(version, key)| version.parse::<u64>().ok().map(|v| (v, key)))
        .max_by_key(|(version, _)| *version)
        .map(|(_, key)| key)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn latest_key_version_picks_highest_numeric_version() {
        let mut keys = HashMap::new();
        keys.insert("1".to_string(), KeyVersion { public_key: B64.encode([1u8; 32]) });
        keys.insert("3".to_string(), KeyVersion { public_key: B64.encode([3u8; 32]) });
        keys.insert("not-a-version".to_string(), KeyVersion { public_key: B64.encode([9u8; 32]) });

        let latest = latest_key_version(&keys).expect("latest key");
        assert_eq!(latest.public_key, B64.encode([3u8; 32]));
    }

    #[test]
    fn public_key_from_key_response_decodes_latest_key() {
        let public_key = [7u8; 32];
        let resp: KeyResp = serde_json::from_value(serde_json::json!({
            "data": {
                "keys": {
                    "1": { "public_key": B64.encode([1u8; 32]) },
                    "2": { "public_key": B64.encode(public_key) }
                }
            }
        }))
        .expect("key response");

        assert_eq!(public_key_from_key_response(&resp, "did-key").expect("public key"), public_key);
    }

    #[test]
    fn public_key_from_key_response_rejects_empty_keys() {
        let resp: KeyResp = serde_json::from_value(serde_json::json!({
            "data": {
                "keys": {}
            }
        }))
        .expect("key response");

        let err = public_key_from_key_response(&resp, "did-key").expect_err("missing key");

        assert!(matches!(err, DIDError::NotFound(_)));
    }

    #[test]
    fn decode_vault_signature_accepts_vault_v1_prefix() {
        let signature = [9u8; 64];
        let encoded = format!("vault:v1:{}", B64.encode(signature));

        assert_eq!(decode_vault_signature(&encoded).expect("signature"), signature);
    }

    #[test]
    fn decode_vault_signature_accepts_rotated_key_versions() {
        let signature = [5u8; 64];
        let encoded = format!("vault:v42:{}", B64.encode(signature));

        assert_eq!(decode_vault_signature(&encoded).expect("signature"), signature);
    }

    #[test]
    fn decode_vault_signature_rejects_unexpected_prefix() {
        let err = decode_vault_signature("not-vault-signature").expect_err("invalid prefix");

        assert!(matches!(err, DIDError::InternalError(_)));
    }
}
