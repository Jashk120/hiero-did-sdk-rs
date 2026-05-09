use hiero_sdk::{AccountId, Client, PrivateKey};
use hiero_did_core::DIDError;

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
