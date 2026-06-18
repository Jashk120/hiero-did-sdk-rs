use hiero_did_core::{
    DIDError,
    Signer,
};
use hiero_sdk::{
    AccountId,
    Client,
    PrivateKey,
};

// ── LocalSigner ───────────────────────────────────────────────────────────────

/// A [`Signer`] backed by a local `hiero_sdk::PrivateKey`.
///
/// Used wherever HCS transactions need access-controlled topic signing.
/// Keeps `PrivateKey` usage contained to the `hcs` crate boundary.
pub struct LocalSigner {
    private_key: PrivateKey,
}

impl LocalSigner {
    pub fn new(private_key: PrivateKey) -> Self {
        Self { private_key }
    }
}

impl Signer for LocalSigner {
    fn public_key_bytes(&self) -> Vec<u8> {
        self.private_key.public_key().to_bytes_raw()
    }

    fn sign_bytes(&self, message: &[u8]) -> Result<Vec<u8>, DIDError> {
        Ok(self.private_key.sign(message).to_vec())
    }
}

// ── HcsClient ────────────────────────────────────────────────────────────────

pub struct HcsClient {
    pub inner: Client,
}

impl HcsClient {
    pub fn for_testnet() -> Self {
        Self { inner: Client::for_testnet() }
    }

    pub fn for_mainnet() -> Self {
        Self { inner: Client::for_mainnet() }
    }

    pub fn set_operator(&self, account_id: AccountId, private_key: PrivateKey) {
        self.inner.set_operator(account_id, private_key);
    }

    pub fn for_testnet_with_operator(
        account_id: AccountId,
        private_key: PrivateKey,
    ) -> Result<Self, DIDError> {
        let client = Client::for_testnet();
        client.set_operator(account_id, private_key);
        Ok(Self { inner: client })
    }
}
