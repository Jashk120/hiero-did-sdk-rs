//! Shared local-node test harness.
//!
//! # Prerequisites
//! Start the local Hedera node before running integration tests:
//!   `docker compose up -d`  (from repo root or local-node directory)
//!
//! The health-check gate in [`setup`] will fail fast with a clear message
//! instead of letting all tests individually time out.

#![allow(dead_code)]

use std::env;
use std::sync::Arc;
use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

use dotenvy::{
    from_filename,
    from_filename_override,
};
use hiero_did_sdk::client::{
    HederaClientConfiguration,
    HederaNetwork,
    NetworkConfig,
};
use hiero_did_sdk::{
    HieroDidSdk,
    core,
    hcs,
};
pub use hiero_did_utils::polling::poll_until;
use hiero_sdk::PrivateKey;

/// All context needed for one integration test run.
pub struct Ctx {
    pub sdk: HieroDidSdk,
    pub signer: Arc<dyn core::Signer>,
    pub network: HederaNetwork,
    pub operator_id: String,
}

/// Returns a unique suffix based on wall-clock nanoseconds — enough to avoid
/// topic/DID collisions across tests in a single run.
pub fn unique_tag(prefix: &str) -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
    uuid::Uuid::new_v4().to_string();
    format!("{prefix}-{nanos}")
}

/// Load `.env.local` (preferred) then `.env` for credentials, then build a
/// shared [`Ctx`].
///
/// Returns `None` and prints a clear skip message if the env is missing or
/// the local node is unreachable — callers should `return` immediately so the
/// test is silently skipped rather than panicking.
pub fn setup() -> Option<Ctx> {
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");

    let operator_id = env::var("HEDERA_ACCOUNT_ID").ok()?;
    let operator_key = env::var("HEDERA_PRIVATE_KEY").ok()?;
    let network_str = env::var("HEDERA_NETWORK").unwrap_or_else(|_| "local".to_string());

    let network = match network_str.as_str() {
        "mainnet" => HederaNetwork::Mainnet,
        "testnet" => HederaNetwork::Testnet,
        "previewnet" => HederaNetwork::Previewnet,
        _ => HederaNetwork::LocalNode,
    };

    let config = HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network: network.clone(),
            operator_id: operator_id.clone(),
            operator_key: operator_key.clone(),
        }],
    };

    let sdk = HieroDidSdk::from_config(config, None)
        .map_err(|e| eprintln!("[harness] Failed to build SDK: {e}"))
        .ok()?;

    // Health-check gate: can we get a client at all?
    sdk.client_service()
        .get_client(None)
        .map_err(|e| eprintln!("[harness] Local node not reachable — start it first. Error: {e}"))
        .ok()?;

    let private_key = PrivateKey::from_str_der(&operator_key)
        .map_err(|e| eprintln!("[harness] Invalid HEDERA_PRIVATE_KEY: {e}"))
        .ok()?;

    let signer: Arc<dyn core::Signer> = Arc::new(hcs::LocalSigner::new(private_key));

    Some(Ctx { sdk, signer, network, operator_id })
}

/// Poll `resolve_did` until the DID resolves successfully, or timeout.
///
/// Uses `poll_until` from the utils crate. Pass `timeout_secs = 30` for
/// standard mirror propagation waits. Returns the resolution result on
/// success, or `None` on timeout.
pub async fn wait_for_did(
    sdk: &HieroDidSdk,
    did: &str,
    timeout_secs: u64,
) -> Option<hiero_did_sdk::core::DIDResolution> {
    poll_until(
        || {
            let did = did.to_string();
            async move { sdk.resolve_did(&did, None).await.ok() }
        },
        timeout_secs,
        1000,
    )
    .await
}

// ---------------------------------------------------------------------------
// Shared mock signer — represents any "external" signer (Vault, HSM, etc.)
// Tests using this prove the SDK's _with_signer plumbing wires correctly.
// They do NOT prove real Vault auth, token handling, or HSM encoding.
// See: graduation PR description for follow-up real-Vault test suite scope.
// ---------------------------------------------------------------------------

/// A mock external signer backed by a real Ed25519 key.
///
/// This stands in for the VaultSigner (or any other `dyn Signer`) in tests
/// that verify the `_with_signer` HCS write path. It does NOT simulate Vault
/// connectivity, auth, or vault-specific signature encoding.
pub struct MockExternalSigner {
    inner: hcs::LocalSigner,
}

impl MockExternalSigner {
    pub fn new() -> Self {
        let key = PrivateKey::generate_ed25519();
        Self { inner: hcs::LocalSigner::new(key) }
    }

    pub fn as_arc(self) -> Arc<dyn core::Signer> {
        Arc::new(self)
    }
}

impl core::Signer for MockExternalSigner {
    fn public_key_bytes(&self) -> Vec<u8> {
        self.inner.public_key_bytes()
    }

    fn sign_bytes(&self, message: &[u8]) -> Result<Vec<u8>, core::DIDError> {
        self.inner.sign_bytes(message)
    }
}

/// A signer that always returns an error — for testing error propagation.
pub struct FailingSigner;

impl core::Signer for FailingSigner {
    fn public_key_bytes(&self) -> Vec<u8> {
        vec![1u8; 32]
    }

    fn sign_bytes(&self, _: &[u8]) -> Result<Vec<u8>, core::DIDError> {
        Err(core::DIDError::InternalError("mock signing failure".into()))
    }
}

/// A signer that returns wrong-length signature bytes — for testing bad-sig propagation.
pub struct MalformedSigner;

impl core::Signer for MalformedSigner {
    fn public_key_bytes(&self) -> Vec<u8> {
        vec![1u8; 32]
    }

    fn sign_bytes(&self, _: &[u8]) -> Result<Vec<u8>, core::DIDError> {
        // 63 bytes instead of the required 64 — should be rejected downstream
        Ok(vec![0xAB; 63])
    }
}
