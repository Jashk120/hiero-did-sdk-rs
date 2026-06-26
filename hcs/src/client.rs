use std::collections::HashMap;
use std::str::FromStr;

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

    /// Build a client pointed at the local Hedera node (hedera-local-node).
    ///
    /// Node address defaults to `127.0.0.1:35211` and mirror gRPC to
    /// `127.0.0.1:38081`, overridable via `HEDERA_NODE_ADDRESS` /
    /// `HEDERA_MIRROR_NODE_ADDRESS` env vars — the same as
    /// `HederaClientService` uses for `HederaNetwork::LocalNode`.
    pub fn for_local_node_with_operator(
        account_id: AccountId,
        private_key: PrivateKey,
    ) -> Result<Self, DIDError> {
        let node_addr = std::env::var("HEDERA_NODE_ADDRESS")
            .unwrap_or_else(|_| "127.0.0.1:35211".to_string());
        let mirror_addr = std::env::var("HEDERA_MIRROR_NODE_ADDRESS")
            .unwrap_or_else(|_| "127.0.0.1:38081".to_string());

        let mut nodes = HashMap::new();
        nodes.insert(
            node_addr,
            AccountId::from_str("0.0.3")
                .map_err(|e| DIDError::InvalidArgument(format!("Invalid node account: {e}")))?,
        );
        let client = Client::for_network(nodes)
            .map_err(|e| DIDError::InternalError(format!("Failed to build local client: {e}")))?;
        client.set_mirror_network(vec![mirror_addr]);
        client.set_operator(account_id, private_key);
        Ok(Self { inner: client })
    }
}