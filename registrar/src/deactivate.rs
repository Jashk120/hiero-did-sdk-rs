use hiero_did_core::signer::Signer;
use hiero_did_core::{DIDError, HederaDid};
use hiero_did_hcs::HcsTopic;
use hiero_did_lifecycle::{LifecycleBuilder, LifecycleRunner, LifecycleRunnerOptions};
use hiero_did_messages::DIDDeactivateMessage;
use hiero_did_signer::InternalSigner;
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
    let signer = InternalSigner::from_raw_bytes(private_key_bytes)?;
    deactivate_did_with_signer(client, did, &signer).await
}

pub async fn deactivate_did_with_signer(
    client: &Client,
    did: HederaDid,
    signer: &dyn Signer,
) -> Result<DeactivateDIDResult, DIDError> {
    let did_str = did.to_string();
    let topic_id = did
        .topic_id
        .parse()
        .map_err(|e| DIDError::InvalidDid(format!("Cannot parse topic ID from DID: {}", e)))?;

    let message = DIDDeactivateMessage::new(did);

    // 1. Sign (manual because DIDDeactivateMessage doesn't store signature in inner types)
    let msg_bytes = message.message_bytes()?;
    let signature = signer.sign_bytes(&msg_bytes)?;

    // 2. Use runner to wrap the process
    let runner = deactivate_lifecycle()?;
    let mut options = LifecycleRunnerOptions::new(());
    options.signer = Some(signer);
    options.signature = Some(signature.clone());

    let _ = runner.process(message.clone(), options).await?;

    // 3. Submit
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

fn deactivate_lifecycle() -> Result<LifecycleRunner<DIDDeactivateMessage, ()>, DIDError> {
    let builder = LifecycleBuilder::new().sign_with_signer("sign")?;
    Ok(LifecycleRunner::new(builder))
}

#[cfg(test)]
mod tests {
    use super::{deactivate_did, deactivate_did_with_signer};
    use hiero_did_core::{DIDError, HederaDid, Signer, did::Network};
    use hiero_sdk::Client;

    struct TestSigner;

    impl Signer for TestSigner {
        fn public_key_bytes(&self) -> Vec<u8> {
            vec![1u8; 32]
        }

        fn sign_bytes(&self, _message: &[u8]) -> Result<Vec<u8>, DIDError> {
            Ok(vec![2u8; 64])
        }
    }

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

    #[tokio::test]
    async fn deactivate_with_signer_rejects_invalid_topic_id() {
        let client = Client::for_testnet();
        let did = HederaDid::new(Network::Testnet, "abc".to_string(), "bad-topic".to_string());
        let signer = TestSigner;

        let err = match deactivate_did_with_signer(&client, did, &signer).await {
            Ok(_) => panic!("must fail"),
            Err(e) => e,
        };

        assert!(matches!(err, DIDError::InvalidDid(_)));
    }
}
