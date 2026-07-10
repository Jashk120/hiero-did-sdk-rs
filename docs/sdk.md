# Hiero SDK Usage Examples

The `HieroDidSdk` struct provides a unified, top-level entrypoint for working with the Hedera DID SDK. It wraps network clients, caching, HCS topic configurations, cryptographic state machine (CSM) operations, and AnonCreds registries to shield developers from lower-level orchestration boilerplate.

---

## 1. Initialization

Before using the SDK, configure your Hedera operator credentials and instantiate the SDK handler.

```rust
use hiero_did_sdk::{
    HieroDidSdk,
    client::{HederaClientConfiguration, NetworkConfig, HederaNetwork},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define network config
    let config = HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network: HederaNetwork::Testnet,
            operator_id: "0.0.12345".to_string(),
            operator_key: "302e02010030...".to_string(), // Operator DER private key string
        }],
    };

    // 2. Initialize SDK (optionally provide a memory/file cache)
    let sdk = HieroDidSdk::from_config(config, None)?;
    
    Ok(())
}
```

---

## 2. Standard DID Lifecycle

Create, resolve, and deactivate DIDs directly.

```rust
use hiero_did_sdk::{
    HieroDidSdk,
    core::did::Network,
};

async fn run_lifecycle(sdk: &HieroDidSdk) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a DID (automatically constructs HCS topic & publishes did message)
    let create_result = sdk.create_did(None, Network::Testnet, None).await?;
    let did = create_result.did;
    println!("Created DID: {}", did);

    // 2. Resolve the DID Document
    let resolution = sdk.resolve_did(&did, None).await?;
    println!("Resolved Document ID: {}", resolution.did_document.id);

    // 3. Deactivate the DID using the ownership private key
    let raw_key = [0u8; 32]; // Key bytes matching the DID identifier
    sdk.deactivate_did(None, create_result.hedera_did, &raw_key).await?;
    
    Ok(())
}
```

---

## 3. Vault-Backed Signer Integration

Use HashiCorp Vault to securely delegate cryptographic operations. The `HieroDidSdk` provides modular `_with_signer` equivalents for DID operations.

```rust
use hiero_did_sdk::{
    HieroDidSdk,
    core::did::Network,
    signer::{VaultSignerConfig, VaultAuth},
};

async fn run_vault_signing(sdk: &HieroDidSdk) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define Vault Configuration
    let vault_config = VaultSignerConfig::new(
        "http://localhost:8200",               // Vault Server address
        VaultAuth::Token("s.xxxxxx".into()),   // Authentication method (Token, AppRole, or Userpass)
        "my-transit-key-name",                 // Key identifier in transit engine
    );

    // 2. Create the Vault Signer
    let vault_signer = sdk.new_vault_signer(vault_config)?;

    // 3. Create a DID with the Vault Signer delegator
    let result = sdk
        .create_did_with_signer(None, Network::Testnet, None, &vault_signer)
        .await?;

    println!("Vault-secured DID: {}", result.did);
    Ok(())
}
```

---

## 4. Cryptographic State Machine (CSM)

For architectures involving separation of concerns (e.g., preparation on an application server, signing in a browser/secure enclave, and submission by an operator), use the Cryptographic State Machine (CSM) flow:

```rust
use hiero_did_sdk::{
    HieroDidSdk,
    core::did::Network,
    registrar::CsmPrepareOptions,
};

async fn run_csm_flow(sdk: &HieroDidSdk) -> Result<(), Box<dyn std::error::Error>> {
    let public_key = vec![1u8; 32];

    // 1. Prepare creation request (builds HCS topic and state machine)
    let signing_req = sdk
        .prepare_create_did_csm(None, Network::Testnet, public_key, None)
        .await?;
        
    // 2. Extract raw bytes to sign externally 
    let bytes_to_sign = signing_req.bytes_to_sign;
    
    // (Perform signing offline/externally)
    let signature = external_sign_bytes(&bytes_to_sign); 

    // 3. Convert signature into a submit request
    let submit_req = signing_req.into_submit_request(signature)?;

    // 4. Submit to Hedera
    let result = sdk.submit_create_did_csm(None, submit_req).await?;
    println!("CSM DID registered: {}", result.did);

    Ok(())
}
```

---

## 5. AnonCreds Registry

Register and resolve AnonCreds objects (Schemas and Credential Definitions) using the built-in registry wrapper:

```rust
use std::sync::Arc;
use hiero_did_sdk::{
    HieroDidSdk,
    anoncreds::{AnonCredsSchema, AnonCredsCredentialDefinition, CredentialDefinitionValue},
};

async fn run_anoncreds(sdk: &HieroDidSdk) -> Result<(), Box<dyn std::error::Error>> {
    let issuer_did = "did:hedera:testnet:z6MkpTHR8VNsBxRcmSt62E7SdTcv9B8ndv5Tzz6A4j5b2j3m_0.0.12345";
    let my_signer = Arc::new(hiero_did_sdk::hcs::LocalSigner::new(
        hiero_sdk::PrivateKey::generate_ed25519()
    ));

    // 1. Define schema
    let schema = AnonCredsSchema {
        issuer_id: issuer_did.to_string(),
        name: "DriverLicense".to_string(),
        version: "1.0".to_string(),
        attr_names: vec!["first_name".to_string(), "last_name".to_string()],
    };

    // 2. Register schema using AnonCreds registry
    let schema_id = sdk
        .anoncreds()
        .register_schema(None, schema, my_signer.clone())
        .await?;

    println!("AnonCreds Schema ID registered: {}", schema_id);
    
    // 3. Retrieve schema
    let retrieved_schema = sdk.anoncreds().get_schema(&schema_id).await?;
    assert_eq!(retrieved_schema.name, "DriverLicense");

    Ok(())
}
```
