# Create DID Guide

This guide covers DID creation flows only.

## What `create_did` Does

`hiero_did_registrar::create::create_did` performs:

1. Generate a new Ed25519 keypair for the DID.
2. Create a new Hedera Consensus Service topic.
3. Construct a DID owner event payload.
4. Sign the payload with the new private key.
5. Submit the signed payload to the created topic.
6. Return DID + raw key bytes.

## API

```rust
pub async fn create_did(
    client: &hiero_sdk::Client,
    network: hiero_did_core::did::Network,
    controller: Option<String>,
) -> Result<CreateDIDResult, hiero_did_core::DIDError>
```

Return type:

- `did`: constructed `HederaDid`
- `private_key_bytes`: 32-byte Ed25519 private key (raw)
- `public_key_bytes`: 32-byte Ed25519 public key (raw)

## Creation Path 1 (Recommended): High-level Registrar API

Use this when you want one-call DID creation.

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
    println!("Public key bytes: {}", created.public_key_bytes.len());

    Ok(())
}
```

## Creation Path 2: High-level With Explicit Controller

If you want the controller field in DID owner event set to a custom value:

```rust
let controller = Some("did:hedera:testnet:...".to_string());
let created = create_did(&client, Network::Testnet, controller).await?;
```

If `controller` is `None`, the SDK uses the created DID itself as controller.

## Creation Path 3: Manual/Step-by-step Building Blocks

Use this if you need custom creation orchestration.

1. Create topic using `hiero_did_hcs::HcsTopic::create`.
2. Build your DID with `HederaDid::new`.
3. Build owner message with `DIDOwnerMessage::new`.
4. Get message bytes using `message_bytes()`.
5. Sign using `InternalSigner`.
6. Serialize envelope via `to_payload()`.
7. Submit payload via `HcsTopic::submit`.

This manual flow gives maximum control, but `create_did` already implements it safely.

## Security Notes

- `private_key_bytes` is raw private key material. Persist securely.
- Do not log private keys.
- Consider encrypted at-rest storage for returned key bytes.

## Errors You May See

All return `DIDError` variants:

- `InternalError`: Hedera topic create/submit failure.
- `SerializationError`: payload serialization failure.
- `InvalidArgument`: malformed key bytes, invalid inputs.

## After Creation

To resolve the created DID:

1. Wait for mirror-node propagation.
2. Use `MirrorNodeClient::get_topic_messages`.
3. Use `DidDocumentBuilder::from(messages).resolve(&did)`.

See `docs/api-reference.md` for full resolver usage.
