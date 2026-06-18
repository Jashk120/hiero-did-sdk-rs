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

const SUBMIT_MAX_RETRIES: u32 = 3;
const SUBMIT_BASE_BACKOFF_MS: u64 = 500;

pub struct CreateDIDResult {
    pub did: HederaDid,
    pub private_key_bytes: Vec<u8>,
    pub public_key_bytes: Vec<u8>,
}

pub struct CreateDIDWithSignerResult {
    pub did: HederaDid,
    pub public_key_bytes: Vec<u8>,
}

/// Creates a new did:hedera DID.
///
/// # Errors
/// If the HCS topic was created but a later step (signing or submission)
/// fails, the returned `DIDError::InternalError` message is prefixed with
/// `orphaned_topic=<TOPIC_ID>` so the stranded topic can be identified from
/// logs. This is a stopgap pending a possible dedicated error variant
/// (tracked separately — see discord thread on DIDError/JS ErrorCodes parity).
pub async fn create_did(
    client: &Client,
    network: Network,
    controller: Option<String>,
) -> Result<CreateDIDResult, DIDError> {
    let hiero_private_key = PrivateKey::generate_ed25519();
    let hiero_public_key = hiero_private_key.public_key();
    let private_key_bytes = hiero_private_key.to_bytes_raw();
    let public_key_bytes = hiero_public_key.to_bytes_raw();

    let topic_id = HcsTopic::create(client).await?;
    let topic_id_str = format!("{}", topic_id);

    let base58_key = KeysUtility::from_bytes(public_key_bytes.clone()).to_base58();
    let did = HederaDid::new(network, base58_key, topic_id_str);

    let message = DIDOwnerMessage::new(did.clone(), public_key_bytes.clone(), controller);
    let signer = InternalSigner::from_raw_bytes(&private_key_bytes)?;

    finalize_did(client, topic_id, message, &signer)
        .await
        .map_err(|e| orphan(topic_id, e))?;

    Ok(CreateDIDResult {
        did,
        private_key_bytes,
        public_key_bytes,
    })
}

/// Creates a DID using an externally-managed signer.
///
/// # Errors
/// Same orphaned-topic behavior as `create_did` — see that doc comment.
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

    finalize_did(client, topic_id, message, signer)
        .await
        .map_err(|e| orphan(topic_id, e))?;

    Ok(CreateDIDWithSignerResult {
        did,
        public_key_bytes,
    })
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Signs and submits the DIDOwner message. Any error here means the caller's
/// topic is now orphaned.
async fn finalize_did(
    client: &Client,
    topic_id: TopicId,
    message: DIDOwnerMessage,
    signer: &dyn Signer,
) -> Result<(), DIDError> {
    let payload = sign_message(message, signer).await?;
    submit_with_retry(client, topic_id, payload).await
}

/// Prefixes an error's message with `orphaned_topic=<TOPIC_ID>` so the
/// stranded topic is greppable/parseable from logs. If the message is
/// already prefixed this way (e.g. bubbled up from `submit_with_retry`),
/// it's left unchanged rather than double-prefixed.
fn orphan(topic_id: TopicId, e: DIDError) -> DIDError {
    let msg = e.to_string();
    if msg.contains("orphaned_topic=") {
        return e;
    }
    DIDError::InternalError(format!("orphaned_topic={} reason={}", topic_id, msg))
}

async fn sign_message(
    message: DIDOwnerMessage,
    signer: &dyn Signer,
) -> Result<String, DIDError> {
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

/// Attempts to submit `payload` to `topic_id`, retrying with exponential
/// backoff (500ms, 1000ms, 2000ms). Retries unconditionally for now —
/// transient-vs-permanent classification is blocked on `hiero_sdk::Error`
/// shape, separate from this fix.
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
                let msg = e.to_string();

                if msg.contains("submit_receipt_failed") {
                    // Transaction may have reached consensus; resubmitting
                    // risks a duplicate message. Don't retry — surface
                    // immediately as ambiguous/orphaned.
                    tracing::error!(
                        topic_id = %topic_id,
                        error = %e,
                        "HCS submit receipt unknown — not retrying to avoid duplicate"
                    );
                    return Err(DIDError::InternalError(format!(
                        "orphaned_topic={} reason={}", topic_id, e
                    )));
                }

                // submit_execute_failed (or unrecognized) — safe to retry,
                // nothing was sent.
                let backoff_ms = SUBMIT_BASE_BACKOFF_MS * (1 << attempt);
                tracing::warn!(
                    attempt = attempt + 1,
                    max = SUBMIT_MAX_RETRIES,
                    backoff_ms,
                    topic_id = %topic_id,
                    error = %e,
                    "HCS submit execute failed, retrying"
                );
                last_err = Some(e);
                sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }

    tracing::error!(topic_id = %topic_id, "HCS submit failed after all retries — topic is orphaned");
    Err(DIDError::InternalError(format!(
        "orphaned_topic={} reason={}",
        topic_id,
        last_err.map(|e| e.to_string()).unwrap_or_else(|| "unknown".into())
    )))
}
fn create_lifecycle() -> Result<LifecycleRunner<DIDOwnerMessage, ()>, DIDError> {
    let builder = LifecycleBuilder::new().sign_with_signer("sign")?;
    Ok(LifecycleRunner::new(builder))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The error produced by `HcsTopic::submit` when the receipt fetch fails is
    /// `DIDError::InternalError("submit_receipt_failed: <details>")`.
    /// `thiserror` renders that as `"Internal error: submit_receipt_failed: <details>"`.
    /// This test proves that `contains()` matches while `starts_with()` would not.
    #[test]
    fn submit_receipt_error_is_detected_by_contains() {
        let inner =
            DIDError::InternalError("submit_receipt_failed: status UNKNOWN".into());
        let rendered = inner.to_string();

        // The rendered form has the thiserror prefix, so starts_with would fail:
        assert!(
            !rendered.starts_with("submit_receipt_failed"),
            "starts_with should NOT match (thiserror prefix present): {rendered}"
        );
        // ...but contains matches correctly:
        assert!(
            rendered.contains("submit_receipt_failed"),
            "contains must match the inner marker: {rendered}"
        );
    }

    /// `orphan()` should detect an already-prefixed error and return it unchanged,
    /// even when the prefix is behind the thiserror `"Internal error: "` wrapper.
    #[test]
    fn orphan_deduplicates_already_prefixed_error() {
        let topic_id: TopicId = "0.0.12345".parse().unwrap();

        // Simulate an error that already carries the orphaned_topic tag
        // (e.g. bubbled up from submit_with_retry → finalize_did).
        let already_tagged = DIDError::InternalError(
            "orphaned_topic=0.0.12345 reason=boom".into(),
        );

        let result = orphan(topic_id, already_tagged);
        let msg = result.to_string();

        // Should NOT double-prefix:
        assert!(
            !msg.contains("orphaned_topic=0.0.12345 reason=Internal error: orphaned_topic="),
            "orphan() must not double-prefix: {msg}"
        );
        // The original message should be preserved as-is:
        assert!(
            msg.contains("orphaned_topic=0.0.12345 reason=boom"),
            "original payload must be preserved: {msg}"
        );
    }

    /// A fresh error without the orphaned_topic tag should be wrapped.
    #[test]
    fn orphan_wraps_fresh_error() {
        let topic_id: TopicId = "0.0.99999".parse().unwrap();
        let fresh = DIDError::InternalError("connection refused".into());
        let result = orphan(topic_id, fresh);
        let msg = result.to_string();

        assert!(
            msg.contains("orphaned_topic=0.0.99999"),
            "must contain orphaned_topic tag: {msg}"
        );
        assert!(
            msg.contains("connection refused"),
            "must contain original reason: {msg}"
        );
    }
}