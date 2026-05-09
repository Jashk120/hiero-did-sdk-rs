use hiero_did_core::{DIDError, HederaDid, KeysUtility};
use hiero_did_core::did::Network;
use hiero_did_messages::DIDOwnerMessage;
use hiero_did_signer::InternalSigner;
use hiero_sdk::{Client, PrivateKey};
use hiero_did_hcs::HcsTopic;


pub struct CreateDIDResult {
    pub did: HederaDid,
    /// Raw 32-byte ed25519 private key — caller must store this securely
    pub private_key_bytes: Vec<u8>,
    /// Raw 32-byte ed25519 public key
    pub public_key_bytes: Vec<u8>,
}

/// Creates a new did:hedera DID.
///
/// Steps:
/// 1. Generate a new ed25519 keypair for the DID
/// 2. Create an HCS topic (this becomes the DID's topic ID)
/// 3. Build and sign the DIDOwner message
/// 4. Submit the signed message to the topic
/// 5. Return the DID and keypair
pub async fn create_did(
    client: &Client,
    network: Network,
    controller: Option<String>,
) -> Result<CreateDIDResult, DIDError> {
    // 1. Generate DID keypair
    let hiero_private_key = PrivateKey::generate_ed25519();
    let hiero_public_key = hiero_private_key.public_key();
    let private_key_bytes = hiero_private_key.to_bytes_raw();
    let public_key_bytes = hiero_public_key.to_bytes_raw();

    // 2. Create HCS topic — this topic ID becomes part of the DID string
    let topic_id = HcsTopic::create(client).await?;
    let topic_id_str = format!("{}", topic_id);

    // 3. Build DID now that we have the topic ID
    let base58_key = KeysUtility::from_bytes(public_key_bytes.clone()).to_base58();
    let did = HederaDid::new(network, base58_key, topic_id_str);

    // 4. Build the DIDOwner message
    let message = DIDOwnerMessage::new(
        did.clone(),
        public_key_bytes.clone(),
        controller,
    );

    // 5. Sign with the DID private key
    let signer = InternalSigner::from_raw_bytes(&private_key_bytes)?;
    let msg_bytes = message.message_bytes()?;
    let signature = signer.sign(&msg_bytes);

    // 6. Build the signed payload
    let payload = message.to_payload(&signature)?;

    // 7. Submit to HCS
    HcsTopic::submit(client, topic_id, payload).await?;

    Ok(CreateDIDResult {
        did,
        private_key_bytes,
        public_key_bytes,
    })
}
