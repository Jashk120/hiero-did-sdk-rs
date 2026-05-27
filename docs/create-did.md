# Create DID Guide

This guide covers DID creation with `hiero-did-registrar`.

## API

Default local-key creation:

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

External signer creation:

```rust
pub async fn create_did_with_signer(
    client: &hiero_sdk::Client,
    network: hiero_did_core::did::Network,
    controller: Option<String>,
    signer: &dyn hiero_did_core::Signer,
) -> Result<CreateDIDWithSignerResult, hiero_did_core::DIDError>
```

`CreateDIDWithSignerResult`:

- `did`: created `HederaDid`
- `public_key_bytes`: raw 32-byte Ed25519 public key

## What `create_did` Does

1. Generates a new Ed25519 keypair.
2. Creates a new HCS topic.
3. Builds a `did:hedera` from network + public key + topic ID.
4. Builds a DID owner message.
5. Signs serialized `HcsMessage` bytes.
6. Submits the signed envelope to the topic.
7. Returns DID + key bytes.

## What `create_did_with_signer` Does

1. Reads public key bytes from the provided `Signer`.
2. Creates a new HCS topic.
3. Builds a `did:hedera` from network + signer public key + topic ID.
4. Builds a DID owner message.
5. Signs serialized `HcsMessage` bytes through `Signer::sign_bytes`.
6. Submits the signed envelope to the topic.
7. Returns DID + public key bytes. Private key bytes are not returned because key custody is external.

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

## External Signer Example

```rust
use hiero_did_core::did::Network;
use hiero_did_registrar::create::create_did_with_signer;
use hiero_did_signer::InternalSigner;
use hiero_sdk::{AccountId, Client, PrivateKey};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let account_id = AccountId::from_str("0.0.12345")?;
    let operator_key = PrivateKey::from_str_der("<DER_PRIVATE_KEY>")?;

    let client = Client::for_testnet();
    client.set_operator(account_id, operator_key);

    let signer = InternalSigner::from_raw_bytes(&[7u8; 32])?;
    let created = create_did_with_signer(&client, Network::Testnet, None, &signer).await?;

    println!("DID: {}", created.did);
    println!("Topic ID: {}", created.did.topic_id);
    Ok(())
}
```

With the `vault` feature enabled on `hiero-did-signer`, `VaultSigner` can be passed to the same API.

## Controller Behavior

- If `controller` is `Some(value)`, that value is used in owner event data.
- If `controller` is `None`, controller defaults to the DID itself.

## Security Notes

- `private_key_bytes` is sensitive key material.
- Do not log or persist it unencrypted.
- Prefer dedicated key management for production workloads.
- Use `create_did_with_signer`, `update_did_with_signer`, and `deactivate_did_with_signer` when key custody is external, such as HashiCorp Vault transit signing.

## Typical Errors

- `InternalError`: topic create/submit or network failures.
- `SerializationError`: message/envelope serialization failures.
- `InvalidArgument`: malformed key bytes or invalid inputs.

## Related Write Operations

`hiero-did-registrar` also provides:

- `update::update_did(...)`
- `update::update_did_with_signer(...)`
- `deactivate::deactivate_did(...)`
- `deactivate::deactivate_did_with_signer(...)`

See [`api-reference.md`](./api-reference.md) for full write-operation types and options.

## Next Step: Resolve DID

After creation, wait for mirror-node consistency and resolve with:

- `MirrorNodeClient::get_topic_messages`
- `DidDocumentBuilder::from(messages).resolve(&did)`
