use hiero_did_core::Signer;
use hiero_did_core::did::Network;
use hiero_did_core::{DIDError, HederaDid, KeysUtility};
use hiero_did_hcs::HcsTopic;
use hiero_did_lifecycle::{LifecycleBuilder, LifecycleRunner, LifecycleRunnerOptions};
use hiero_did_messages::DIDOwnerMessage;
use hiero_did_signer::InternalSigner;
use hiero_sdk::{Client, PrivateKey, TopicId};
use std::time::Duration;
use tokio::time::sleep;

// ---------------------------------------------------------------------------
// Retry configuration — tweak here if needed, no need for a config struct yet
// ---------------------------------------------------------------------------
const SUBMIT_MAX_RETRIES: u32 = 3;
const SUBMIT_BASE_BACKOFF_MS: u64 = 500;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

pub struct CreateDIDResult {
    pub did: HederaDid,
    /// Raw 32-byte ed25519 private key — caller must store this securely
    pub private_key_bytes: Vec<u8>,
    /// Raw 32-byte ed25519 public key
    pub public_key_bytes: Vec<u8>,
}

pub struct CreateDIDWithSignerResult {
    pub did: HederaDid,
    /// Raw 32-byte ed25519 public key
    pub public_key_bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Creates a new did:hedera DID.
///
/// Steps:
/// 1. Generate a new ed25519 keypair for the DID
/// 2. Create an HCS topic (this becomes the DID's topic ID)
/// 3. Build and sign the DIDOwner message
/// 4. Submit the signed message to the topic (with retry + backoff)
/// 5. Return the DID and keypair
///
/// # Errors
/// Returns `DIDError::OrphanedTopic` if the topic was created but all submit
/// attempts failed. The caller should log the contained `topic_id` for
/// operational visibility.
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

    // 2. Create HCS topic — this topic ID becomes part of the DID string.
    //    Must happen before signing because the topic ID is embedded in the
    //    DID string which is itself included in the signed message.
    let topic_id = HcsTopic::create(client).await?;
    let topic_id_str = format!("{}", topic_id);

    // 3. Build DID now that we have the topic ID
    let base58_key = KeysUtility::from_bytes(public_key_bytes.clone()).to_base58();
    let did = HederaDid::new(network, base58_key, topic_id_str);

    // 4. Build and sign the DIDOwner message
    let message = DIDOwnerMessage::new(did.clone(), public_key_bytes.clone(), controller);
    let signer = InternalSigner::from_raw_bytes(&private_key_bytes)?;
    let payload = sign_message(message, &signer).await?;

    // 5. Submit with retry — topic already exists so we owe it a valid publish
    submit_with_retry(client, topic_id, payload).await?;

    Ok(CreateDIDResult {
        did,
        private_key_bytes,
        public_key_bytes,
    })
}

/// Creates a DID using an externally-managed signer.
///
/// # Errors
/// Returns `DIDError::OrphanedTopic` if the topic was created but all submit
/// attempts failed. The caller should log the contained `topic_id` for
/// operational visibility.
pub async fn create_did_with_signer(
    client: &Client,
    network: Network,
    controller: Option<String>,
    signer: &dyn Signer,
) -> Result<CreateDIDWithSignerResult, DIDError> {
    let public_key_bytes = signer.public_key_bytes();

    let topic_id = HcsTopic::create(client).await?;
    let topic_id_str = format!("{}", topic_id);

    let base58_key = KeysUtility::from_bytes(public_key_bytes.clone()).to_base58();
    let did = HederaDid::new(network, base58_key, topic_id_str);

    let message = DIDOwnerMessage::new(did.clone(), public_key_bytes.clone(), controller);
    let payload = sign_message(message, signer).await?;

    submit_with_retry(client, topic_id, payload).await?;

    Ok(CreateDIDWithSignerResult {
        did,
        public_key_bytes,
    })
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Signs a `DIDOwnerMessage` through the lifecycle runner and returns the
/// serialized payload bytes ready for HCS submission.
async fn sign_message(
    message: DIDOwnerMessage,
    signer: &dyn Signer,
) -> Result<String, DIDError>{
    let runner = create_lifecycle()?;
    let mut options = LifecycleRunnerOptions::new(());
    options.signer = Some(signer);

    let runner_state = runner.process(message, options).await?;
    let signed_message = runner_state.message;
    let signature = signed_message.signature.as_ref().ok_or_else(|| {
        DIDError::InternalError("Lifecycle failed to attach signature".into())
    })?;

    signed_message.to_payload(signature)
}

/// Attempts to submit `payload` to `topic_id`, retrying on transient failures
/// with exponential backoff.
///
/// Backoff schedule (ms): 500 → 1000 → 2000
///
/// On total failure, returns `DIDError::OrphanedTopic` containing the topic ID
/// so the caller can log or alert on the stranded resource.
async fn submit_with_retry(
    client: &Client,
    topic_id: TopicId,
    payload: String,       
) -> Result<(), DIDError> {
    let mut last_err = None;

    for attempt in 0..SUBMIT_MAX_RETRIES {
        match HcsTopic::submit(client, topic_id, payload.clone()).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                let backoff_ms = SUBMIT_BASE_BACKOFF_MS * (1 << attempt); // 500, 1000, 2000
                tracing::warn!(
                    attempt = attempt + 1,
                    max = SUBMIT_MAX_RETRIES,
                    backoff_ms,
                    topic_id = %topic_id,
                    error = %e,
                    "HCS submit failed, retrying"
                );
                last_err = Some(e);
                sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }

    // All retries exhausted — surface a structured error so the caller can
    // record the orphaned topic ID for manual inspection or alerting.
    tracing::error!(
        topic_id = %topic_id,
        "HCS submit failed after all retries — topic is orphaned"
    );
    Err(DIDError::OrphanedTopic {
        topic_id: topic_id.to_string(),
        reason: last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown".into()),
    })
}

fn create_lifecycle() -> Result<LifecycleRunner<DIDOwnerMessage, ()>, DIDError> {
    let builder = LifecycleBuilder::new().sign_with_signer("sign")?;
    Ok(LifecycleRunner::new(builder))
}