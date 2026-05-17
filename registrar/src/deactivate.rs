use hiero_did_core::{DIDError, HederaDid};
use hiero_did_messages::DIDDeactivateMessage;
use hiero_did_signer::InternalSigner;
use hiero_did_hcs::HcsTopic;
use hiero_sdk::Client;

pub struct DeactivateDIDResult {
    pub did: String,
    pub did_document: DeactivatedDIDDocument,
}

/// Tombstoned DID document — per spec verificationMethod is empty after deactivation
pub struct DeactivatedDIDDocument {
    pub id: String,
    pub controller: String,
    pub verification_method: Vec<()>,
}

/// Deactivates an existing did:hedera DID.
///
///
/// Steps:
/// 1. Parse topic ID from the DID string (no new topic created)
/// 2. Build DIDDeactivateMessage
/// 3. Sign with owner's private key
/// 4. Submit to existing HCS topic
/// 5. Return tombstoned document
pub async fn deactivate_did(
    client: &Client,
    did: HederaDid,
    private_key_bytes: &[u8],
) -> Result<DeactivateDIDResult, DIDError> {
    let did_str = did.to_string();

    let topic_id = did.topic_id.parse()
        .map_err(|e| DIDError::InvalidDid(format!("Cannot parse topic ID from DID: {}", e)))?;

    let message = DIDDeactivateMessage::new(did);

    let signer = InternalSigner::from_raw_bytes(private_key_bytes)?;
    let msg_bytes = message.message_bytes()?;
    let signature = signer.sign(&msg_bytes);
    let payload = message.to_payload(&signature)?;

    HcsTopic::submit(client, topic_id, payload).await?;

    Ok(DeactivateDIDResult {
        did_document: DeactivatedDIDDocument {
            id: did_str.clone(),
            controller: did_str.clone(),
            verification_method: vec![],
        },
        did: did_str,
    })
}

#[cfg(test)]
mod tests {
    use super::deactivate_did;
    use hiero_did_core::{did::Network, DIDError, HederaDid};
    use hiero_sdk::Client;

    #[tokio::test]
    async fn deactivate_rejects_invalid_topic_id() {
        let client = Client::for_testnet();
        let did = HederaDid::new(Network::Testnet, "abc".to_string(), "bad-topic".to_string());
        let err = match deactivate_did(&client, did, &[1u8; 32]).await {
            Ok(_) => panic!("must fail"),
            Err(e) => e,
        };
        assert!(matches!(err, DIDError::InvalidDid(_)));
    }
}
