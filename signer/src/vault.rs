use hiero_did_core::error::DIDError;
use hiero_did_core::signer::Signer;

use crate::vault_api::VaultApi;
use crate::vault_config::VaultSignerConfig;

pub struct VaultSigner {
    api: VaultApi,
    key_name: String,
    public_key: Vec<u8>,
}

impl VaultSigner {
    pub fn new(cfg: VaultSignerConfig) -> Result<Self, DIDError> {
        let api = VaultApi::new(&cfg)?;
        let public_key = api.fetch_public_key(&cfg.key_name)?;
        Ok(Self { api, key_name: cfg.key_name, public_key })
    }
}

impl Signer for VaultSigner {
    fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.clone()
    }

    fn sign_bytes(&self, message: &[u8]) -> Result<Vec<u8>, DIDError> {
        self.api.sign_ed25519(&self.key_name, message)
    }
}
