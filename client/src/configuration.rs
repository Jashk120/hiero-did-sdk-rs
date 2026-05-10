use std::collections::HashMap;
use hiero_sdk::AccountId;

/// Named or custom Hedera network.
#[derive(Debug, Clone)]
pub enum HederaNetwork {
    Mainnet,
    Testnet,
    Previewnet,
    LocalNode,
    Custom(HederaCustomNetwork),
}

impl HederaNetwork {
    pub fn name(&self) -> &str {
        match self {
            HederaNetwork::Mainnet => "mainnet",
            HederaNetwork::Testnet => "testnet",
            HederaNetwork::Previewnet => "previewnet",
            HederaNetwork::LocalNode => "localhost",
            HederaNetwork::Custom(c) => &c.name,
        }
    }
}

/// Custom network with explicit consensus nodes and optional mirror nodes.
#[derive(Debug, Clone)]
pub struct HederaCustomNetwork {
    pub name: String,
    /// Map of node address -> AccountId
    pub nodes: HashMap<String, AccountId>,
    /// Mirror node endpoints
    pub mirror_nodes: Option<Vec<String>>,
}

/// Operator + network pairing.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub network: HederaNetwork,
    /// Operator account ID as string e.g. "0.0.1234"
    pub operator_id: String,
    /// Operator private key as string (DER or hex)
    pub operator_key: String,
}

/// Top-level configuration for HederaClientService.
#[derive(Debug, Clone)]
pub struct HederaClientConfiguration {
    pub networks: Vec<NetworkConfig>,
}