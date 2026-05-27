use std::sync::Arc;
use std::time::Duration;

use hiero_did_core::{DIDError, Signer};
use hiero_sdk::{
    Client, Key, TopicCreateTransaction, TopicDeleteTransaction, TopicId, TopicInfoQuery,
    TopicMessageSubmitTransaction, TopicUpdateTransaction,
};
use time::OffsetDateTime;

use crate::hcs::signing::{
    public_key_from_signer, sign_with_error_capture, signing_error_slot, take_signing_error,
};
use crate::shared::wait_for_changes;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TopicInfo {
    pub topic_id: String,
    pub topic_memo: String,
    pub admin_key: Option<String>,
    pub submit_key: Option<String>,
    pub auto_renew_period_seconds: Option<i64>,
    pub auto_renew_account_id: Option<String>,
    pub expiration_time: Option<i64>,
}

pub struct CreateTopicProps {
    pub topic_memo: Option<String>,
    /// Signer whose public key is set as the submit key on the topic.
    pub submit_key_signer: Option<Arc<dyn Signer>>,
    /// Signer whose public key is set as the admin key on the topic.
    pub admin_key_signer: Option<Arc<dyn Signer>>,
    pub auto_renew_period_seconds: Option<i64>,
    pub wait_for_visibility: bool,
    pub wait_timeout_ms: Option<u64>,
}

impl Default for CreateTopicProps {
    fn default() -> Self {
        Self {
            topic_memo: None,
            submit_key_signer: None,
            admin_key_signer: None,
            auto_renew_period_seconds: None,
            wait_for_visibility: false,
            wait_timeout_ms: None,
        }
    }
}

pub struct UpdateTopicProps {
    pub topic_id: TopicId,
    pub topic_memo: Option<String>,
    pub auto_renew_period_seconds: Option<i64>,
    pub expiration_time: Option<OffsetDateTime>,
    /// Must be the current admin key signer to authorize the update.
    pub admin_key_signer: Arc<dyn Signer>,
    pub wait_for_visibility: bool,
    pub wait_timeout_ms: Option<u64>,
}

pub struct SubmitMessageResult {
    pub topic_id: String,
    pub sequence_number: u64,
}

pub struct DeleteTopicProps {
    pub topic_id: TopicId,
    pub wait_for_visibility: bool,
    pub wait_timeout_ms: Option<u64>,
}

// ── HcsTopic ─────────────────────────────────────────────────────────────────

pub struct HcsTopic;

impl HcsTopic {
    /// Create a new HCS topic with no keys or memo.
    pub async fn create(client: &Client) -> Result<TopicId, DIDError> {
        Self::create_with_props(client, CreateTopicProps::default()).await
    }

    /// Create a new HCS topic with a memo only.
    pub async fn create_with_memo(client: &Client, memo: &str) -> Result<TopicId, DIDError> {
        Self::create_with_props(
            client,
            CreateTopicProps {
                topic_memo: Some(memo.to_string()),
                ..Default::default()
            },
        )
        .await
    }

    /// Create a new HCS topic with full props.
    pub async fn create_with_props(
        client: &Client,
        props: CreateTopicProps,
    ) -> Result<TopicId, DIDError> {
        let expected_memo = props.topic_memo.clone();
        let expected_submit_key = props.submit_key_signer.is_some();
        let expected_admin_key = props.admin_key_signer.is_some();
        let expected_auto_renew = props.auto_renew_period_seconds;
        let mut tx = TopicCreateTransaction::new();
        let signing_errors = signing_error_slot();

        if let Some(memo) = props.topic_memo {
            tx.topic_memo(memo);
        }

        if let Some(signer) = props.submit_key_signer {
            let pk = public_key_from_signer(signer.as_ref())?;
            tx.submit_key(pk);
            tx.sign_with(
                pk,
                sign_with_error_capture(Arc::clone(&signer), Arc::clone(&signing_errors)),
            );
        }

        if let Some(signer) = props.admin_key_signer {
            let pk = public_key_from_signer(signer.as_ref())?;
            tx.admin_key(pk);
            tx.sign_with(
                pk,
                sign_with_error_capture(Arc::clone(&signer), Arc::clone(&signing_errors)),
            );
        }

        if let Some(secs) = props.auto_renew_period_seconds {
            tx.auto_renew_period(time::Duration::seconds(secs));
        }

        let response = match tx.execute(client).await {
            Ok(response) => response,
            Err(e) => {
                take_signing_error(&signing_errors)?;
                return Err(DIDError::InternalError(format!(
                    "Failed to create topic: {e}"
                )));
            }
        };
        take_signing_error(&signing_errors)?;

        let receipt = response
            .get_receipt(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to get topic receipt: {e}")))?;

        let topic_id = receipt
            .topic_id
            .ok_or_else(|| DIDError::InternalError("No topic ID in receipt".into()))?;

        if props.wait_for_visibility {
            let c = client.clone();
            wait_for_changes(
                move || {
                    let c2 = c.clone();
                    async move {
                        HcsTopic::get_info(&c2, topic_id)
                            .await
                            .map_err(|e| e.to_string())
                    }
                },
                move |info: &TopicInfo| {
                    let memo_ok = expected_memo
                        .as_ref()
                        .map(|m| info.topic_memo == *m)
                        .unwrap_or(true);
                    let submit_key_ok = if expected_submit_key {
                        info.submit_key.is_some()
                    } else {
                        true
                    };
                    let admin_key_ok = if expected_admin_key {
                        info.admin_key.is_some()
                    } else {
                        true
                    };
                    let auto_renew_ok = expected_auto_renew
                        .map(|s| info.auto_renew_period_seconds == Some(s))
                        .unwrap_or(true);
                    memo_ok && submit_key_ok && admin_key_ok && auto_renew_ok
                },
                props.wait_timeout_ms,
                None,
            )
            .await
            .map_err(DIDError::InternalError)?;
        }

        Ok(topic_id)
    }

    /// Update an existing HCS topic. Requires the current admin key signer.
    pub async fn update(client: &Client, props: UpdateTopicProps) -> Result<(), DIDError> {
        let topic_id = props.topic_id;
        let expected_memo = props.topic_memo.clone();
        let expected_auto_renew = props.auto_renew_period_seconds;
        let expected_exp = props.expiration_time.map(|t| t.unix_timestamp());
        let mut tx = TopicUpdateTransaction::new();
        tx.topic_id(topic_id);

        if let Some(memo) = props.topic_memo {
            tx.topic_memo(memo);
        }

        if let Some(secs) = props.auto_renew_period_seconds {
            tx.auto_renew_period(time::Duration::seconds(secs));
        }

        if let Some(exp) = props.expiration_time {
            tx.expiration_time(exp);
        }

        let signing_errors = signing_error_slot();
        let pk = public_key_from_signer(props.admin_key_signer.as_ref())?;
        tx.sign_with(
            pk,
            sign_with_error_capture(
                Arc::clone(&props.admin_key_signer),
                Arc::clone(&signing_errors),
            ),
        );

        let response = match tx.execute(client).await {
            Ok(response) => response,
            Err(e) => {
                take_signing_error(&signing_errors)?;
                return Err(DIDError::InternalError(format!(
                    "Failed to update topic: {e}"
                )));
            }
        };
        take_signing_error(&signing_errors)?;

        let receipt = response
            .get_receipt(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to get update receipt: {e}")))?;

        if receipt.status != hiero_sdk::Status::Success {
            return Err(DIDError::InternalError(format!(
                "Topic update failed: {:?}",
                receipt.status
            )));
        }

        if props.wait_for_visibility {
            let c = client.clone();

            wait_for_changes(
                move || {
                    let c2 = c.clone();
                    async move {
                        HcsTopic::get_info(&c2, topic_id)
                            .await
                            .map_err(|e| e.to_string())
                    }
                },
                move |info: &TopicInfo| {
                    let memo_ok = expected_memo
                        .as_ref()
                        .map(|m| info.topic_memo == *m)
                        .unwrap_or(true);
                    let auto_renew_ok = expected_auto_renew
                        .map(|s| info.auto_renew_period_seconds == Some(s))
                        .unwrap_or(true);
                    let expiration_ok = expected_exp
                        .map(|ts| info.expiration_time == Some(ts))
                        .unwrap_or(true);

                    memo_ok && auto_renew_ok && expiration_ok
                },
                props.wait_timeout_ms,
                None,
            )
            .await
            .map_err(DIDError::InternalError)?;
        }

        Ok(())
    }

    /// Delete an HCS topic. Topic must have an admin key set.
    pub async fn delete(client: &Client, topic_id: TopicId) -> Result<(), DIDError> {
        Self::delete_with_props(
            client,
            DeleteTopicProps {
                topic_id,
                wait_for_visibility: false,
                wait_timeout_ms: None,
            },
        )
        .await
    }

    /// Delete an HCS topic with optional visibility wait.
    pub async fn delete_with_props(
        client: &Client,
        props: DeleteTopicProps,
    ) -> Result<(), DIDError> {
        let receipt = TopicDeleteTransaction::new()
            .topic_id(props.topic_id)
            .execute(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to delete topic: {e}")))?
            .get_receipt(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to get delete receipt: {e}")))?;

        if receipt.status != hiero_sdk::Status::Success {
            return Err(DIDError::InternalError(format!(
                "Topic delete failed: {:?}",
                receipt.status
            )));
        }

        if props.wait_for_visibility {
            let timeout = Duration::from_millis(props.wait_timeout_ms.unwrap_or(45_000));
            let started = tokio::time::Instant::now();
            loop {
                match HcsTopic::get_info(client, props.topic_id).await {
                    Ok(_) => {}
                    Err(e) => {
                        let err_text = e.to_string();
                        if err_text.contains("NOT_FOUND") || err_text.contains("5 NOT_FOUND") {
                            break;
                        }
                    }
                }

                if started.elapsed() >= timeout {
                    return Err(DIDError::InternalError(format!(
                        "Timed out waiting for topic deletion visibility after {}ms",
                        timeout.as_millis()
                    )));
                }

                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }

        Ok(())
    }

    /// Query topic info via gRPC.
    pub async fn get_info(client: &Client, topic_id: TopicId) -> Result<TopicInfo, DIDError> {
        let info = TopicInfoQuery::new()
            .topic_id(topic_id)
            .execute(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to query topic info: {e}")))?;

        Ok(TopicInfo {
            topic_id: info.topic_id.to_string(),
            topic_memo: info.topic_memo,
            admin_key: info.admin_key.map(|k| key_to_string(&k)),
            submit_key: info.submit_key.map(|k| key_to_string(&k)),
            auto_renew_period_seconds: info.auto_renew_period.map(|d| d.whole_seconds()),
            auto_renew_account_id: info.auto_renew_account_id.map(|a| a.to_string()),
            expiration_time: info.expiration_time.map(|t| t.unix_timestamp()),
        })
    }

    /// Submit a message to an HCS topic.
    pub async fn submit(
        client: &Client,
        topic_id: TopicId,
        message: impl Into<Vec<u8>>,
    ) -> Result<SubmitMessageResult, DIDError> {
        let receipt = TopicMessageSubmitTransaction::new()
            .topic_id(topic_id)
            .message(message)
            .execute(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to submit message: {e}")))?
            .get_receipt(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to get message receipt: {e}")))?;

        Ok(SubmitMessageResult {
            topic_id: topic_id.to_string(),
            sequence_number: receipt.topic_sequence_number,
        })
    }
}

fn key_to_string(key: &Key) -> String {
    match key {
        Key::Single(pk) => pk.to_string_raw(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_topic_props_default_values() {
        let props = CreateTopicProps::default();
        assert!(props.topic_memo.is_none());
        assert!(props.submit_key_signer.is_none());
        assert!(props.admin_key_signer.is_none());
        assert!(props.auto_renew_period_seconds.is_none());
        assert!(!props.wait_for_visibility);
        assert!(props.wait_timeout_ms.is_none());
    }
}
