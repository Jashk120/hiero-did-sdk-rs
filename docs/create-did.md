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

`CreateDIDResult`:

- `did`: created `HederaDid`
- `private_key_bytes`: raw 32-byte Ed25519 private key
- `public_key_bytes`: raw 32-byte Ed25519 public key

## What `create_did` Does

1. Generates a new Ed25519 keypair.
2. Creates a new HCS topic.
3. Builds a `did:hedera` from network + public key + topic ID.
4. Builds a DID owner message.
5. Signs the serialized `HcsMessage` bytes.
6. Submits the signed envelope to the topic.
7. Returns DID + key bytes.

## Example

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
    Ok(())
}
```

## Controller Behavior

- If `controller` is `Some(value)`, that value is used in owner event data.
- If `controller` is `None`, controller defaults to the DID itself.

## Security Notes

- `private_key_bytes` is sensitive key material.
- Do not log or persist it unencrypted.
- Prefer dedicated key management for production workloads.

## Typical Errors

- `InternalError`: topic create/submit or network failures.
- `SerializationError`: message/envelope serialization failures.
- `InvalidArgument`: malformed key bytes or invalid inputs.

## Next Step: Resolve DID

After creation, wait for mirror-node consistency and resolve with:

- `MirrorNodeClient::get_topic_messages`
- `DidDocumentBuilder::from(messages).resolve(&did)`
