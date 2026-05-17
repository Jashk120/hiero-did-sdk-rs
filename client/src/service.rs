use hiero_did_core::DIDError;
use hiero_sdk::{AccountId, Client, Hbar, PrivateKey};
use std::str::FromStr;
use rust_decimal::Decimal;

use crate::configuration::{HederaClientConfiguration, HederaNetwork, NetworkConfig};

const MAX_TRANSACTION_FEE_HBAR: i64 = 2;

/// Optional network selector passed per-call.
#[derive(Debug, Clone, Default)]
pub struct NetworkName {
    pub network_name: Option<String>,
}

impl NetworkName {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            network_name: Some(name.into()),
        }
    }
}

pub struct HederaClientService {
    configuration: HederaClientConfiguration,
}

impl HederaClientService {
    pub fn new(config: HederaClientConfiguration) -> Result<Self, DIDError> {
        if config.networks.is_empty() {
            return Err(DIDError::InvalidArgument(
                "Networks must not be empty".into(),
            ));
        }

        // Validate unique network names
        let names: Vec<&str> = config.networks.iter().map(|n| n.network.name()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        if unique.len() != names.len() {
            return Err(DIDError::InvalidArgument(
                "Network names must be unique".into(),
            ));
        }

        Ok(Self { configuration: config })
    }

    /// Build and return a configured client for the given network name.
    /// If `network_name` is None and only one network is configured, uses that.
    pub fn get_client(&self, network_name: Option<&str>) -> Result<Client, DIDError> {
        let network_config = self.resolve_network_config(network_name)?;
        let client = self.build_client(network_config)?;
        Ok(client)
    }

    /// Run an async operation with a network-specific client.
    /// Client is dropped after the operation completes.
    pub async fn with_client<T, F, Fut>(
        &self,
        network_name: Option<&str>,
        operation: F,
    ) -> Result<T, DIDError>
    where
        F: FnOnce(Client) -> Fut,
        Fut: std::future::Future<Output = Result<T, DIDError>>,
    {
        let client = self.get_client(network_name)?;
        operation(client).await
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn resolve_network_config(
        &self,
        network_name: Option<&str>,
    ) -> Result<&NetworkConfig, DIDError> {
        match network_name {
            None if self.configuration.networks.len() == 1 => {
                Ok(&self.configuration.networks[0])
            }
            None => Err(DIDError::InvalidArgument(
                "network_name required when multiple networks are configured".into(),
            )),
            Some(name) => self
                .configuration
                .networks
                .iter()
                .find(|n| n.network.name() == name)
                .ok_or_else(|| {
                    DIDError::InvalidArgument(format!("Unknown network: {name}"))
                }),
        }
    }

    fn build_client(&self, config: &NetworkConfig) -> Result<Client, DIDError> {
        let operator_id = AccountId::from_str(&config.operator_id)
            .map_err(|e| DIDError::InvalidArgument(format!("Invalid operator ID: {e}")))?;
        let operator_key = PrivateKey::from_str_der(&config.operator_key)
            .map_err(|e| DIDError::InvalidArgument(format!("Invalid operator key: {e}")))?;

        let client = match &config.network {
            HederaNetwork::Mainnet => Client::for_mainnet(),
            HederaNetwork::Testnet => Client::for_testnet(),
            HederaNetwork::Previewnet => Client::for_previewnet(),
            HederaNetwork::LocalNode => Client::for_name("localhost")
                .map_err(|e| DIDError::InternalError(format!("Failed to build local client: {e}")))?,
            HederaNetwork::Custom(custom) => {
                let client = Client::for_network(custom.nodes.clone())
                    .map_err(|e| {
                        DIDError::InternalError(format!("Failed to build custom client: {e}"))
                    })?;
                if let Some(mirror_nodes) = &custom.mirror_nodes {
                    client.set_mirror_network(mirror_nodes.clone());
                }
                client
            }
        };

        client.set_operator(operator_id, operator_key);
        client.set_default_max_transaction_fee(Hbar::from(Decimal::from(MAX_TRANSACTION_FEE_HBAR)));

        Ok(client)
    }
}