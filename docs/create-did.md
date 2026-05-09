# Create DID Guide

This guide covers DID creation with `hiero-did-registrar`.

## API

```rust
pub async fn create_did(
    client: &hiero_sdk::Client,
    network: hiero_did_core::did::Network,
    controller: Option<String>,
) -> Result<CreateDIDResult, hiero_did_core::DIDError>
```

Return type:

- `did`: created `HederaDid`
- `private_key_bytes`: raw 32-byte Ed25519 private key
- `public_key_bytes`: raw 32-byte Ed25519 public key

## What `create_did` Does

1. Generates a new Ed25519 keypair.
2. Creates a new HCS topic.
3. Builds a `did:hedera` from network + public key + topic ID.
4. Builds and signs a DID owner message.
5. Submits signed payload to the topic.
6. Returns DID + key bytes.

## High-Level Usage (Recommended)

```rust
use hiero_did_core::did::Network;
use hiero_did_registrar::create::create_did;
use hiero_sdk::{AccountId, Client, PrivateKey};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let account_id = AccountId::from_str("0.0.12345")?;
    let operator_key = PrivateKey::from_str_der("<DER_PRIVATE_KEY>")?;

    let client = Client::for_testnet();
    client.set_operator(account_id, operator_key);

    let created = create_did(&client, Network::Testnet, None).await?;

    println!("DID: {}", created.did);
    println!("Topic ID: {}", created.did.topic_id);
    println!("Public key len: {}", created.public_key_bytes.len());

    Ok(())
}
```

## Controller Behavior

- If `controller` is `Some(value)`, that value is used as controller in owner event data.
- If `controller` is `None`, controller defaults to the DID itself.

## Manual Building Blocks (Advanced)

For custom orchestration:

1. `hiero_did_hcs::HcsTopic::create`
2. `hiero_did_core::did::HederaDid::new`
3. `hiero_did_messages::DIDOwnerMessage::new`
4. `message_bytes()` + `hiero_did_signer::InternalSigner::sign`
5. `to_payload()`
6. `hiero_did_hcs::HcsTopic::submit`

Use this only if you need full control over creation flow.

## Security Notes

- `private_key_bytes` is sensitive secret material.
- Do not log private key bytes.
- Store keys encrypted at rest with access controls.

## Typical Errors

- `InternalError`: topic create/submit or network failures.
- `SerializationError`: message/envelope serialization failures.
- `InvalidArgument`: malformed key bytes or invalid inputs.

## Next Step: Resolve DID

After creation, wait briefly for mirror-node consistency, then resolve with:

- `MirrorNodeClient::get_topic_messages`
- `DidDocumentBuilder::from(messages).resolve(&did)`
