# Client-Side Message Signing

CSM lets the SDK prepare DID operation bytes while an external client, wallet, or hardware key signs those bytes.

## Flow

1. Call a `prepare_*_did_csm` function.
2. Send `CsmSigningRequest.message_bytes` to the external signer.
3. Convert the signing request into a submit request with `into_submit_request(signature)`.
4. Call the matching `submit_*_did_csm` function.

The SDK validates the CSM state before submit:

- state version
- deterministic `request_id`
- exact rebuilt message bytes
- optional expiry
- 64-byte Ed25519 signature shape
- signature verification against the expected public key

## Create

```rust
use hiero_did_core::did::Network;
use hiero_did_registrar::{prepare_create_did_csm, submit_create_did_csm};
use hiero_did_signer::InternalSigner;

# async fn example(client: &hiero_sdk::Client) -> Result<(), Box<dyn std::error::Error>> {
let signer = InternalSigner::from_bytes(&[7u8; 32])?;
let request = prepare_create_did_csm(
    client,
    Network::Testnet,
    signer.verifying_key_bytes(),
    None,
)
.await?;

let signature = signer.sign(&request.message_bytes);
let submit_request = request.into_submit_request(signature)?;
let result = submit_create_did_csm(client, submit_request).await?;

println!("submitted {} to topic {}", result.did, result.topic_id);
# Ok(())
# }
```

## Update

`prepare_update_did_csm` returns a batch request. Sign each request's `message_bytes` in order and pass the matching signatures to `into_submit_request`.

```rust
use hiero_did_registrar::{
    AddService, DIDUpdateOperation, prepare_update_did_csm, submit_update_did_csm,
};

# async fn example(
#     client: &hiero_sdk::Client,
#     did: hiero_did_core::HederaDid,
#     signer: &hiero_did_signer::InternalSigner,
# ) -> Result<(), Box<dyn std::error::Error>> {
let request = prepare_update_did_csm(
    did.clone(),
    vec![DIDUpdateOperation::AddService(AddService {
        id: format!("{did}#linked-domain"),
        service_type: "LinkedDomains".to_string(),
        service_endpoint: "https://example.com".to_string(),
    })],
)?;

let signatures = request
    .requests
    .iter()
    .map(|item| signer.sign(&item.message_bytes))
    .collect();

let submit_request = request.into_submit_request(signatures)?;
let result = submit_update_did_csm(client, submit_request).await?;

assert_eq!(result.operations_applied, 1);
# Ok(())
# }
```

## Deactivate

```rust
use hiero_did_registrar::{prepare_deactivate_did_csm, submit_deactivate_did_csm};

# async fn example(
#     client: &hiero_sdk::Client,
#     did: hiero_did_core::HederaDid,
#     signer: &hiero_did_signer::InternalSigner,
# ) -> Result<(), Box<dyn std::error::Error>> {
let request = prepare_deactivate_did_csm(did)?;
let signature = signer.sign(&request.message_bytes);
let submit_request = request.into_submit_request(signature)?;
let result = submit_deactivate_did_csm(client, submit_request).await?;

assert_eq!(result.operation, "delete");
# Ok(())
# }
```

## Expiry

Use the `_with_options` variants to add an expiry timestamp:

```rust
use hiero_did_registrar::CsmPrepareOptions;

let options = CsmPrepareOptions {
    expires_at_unix: Some(1_893_456_000),
};
```

Expired requests are rejected before the SDK submits anything to HCS.
