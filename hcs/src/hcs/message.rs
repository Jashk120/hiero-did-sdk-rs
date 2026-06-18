use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use hiero_did_core::{
    DIDError,
    Signer,
};
use hiero_sdk::{
    Client,
    TopicId,
    TopicMessageQuery,
    TopicMessageSubmitTransaction,
};
use time::OffsetDateTime;

use crate::cache::HcsCacheService;
use crate::hcs::signing::{
    public_key_from_signer,
    sign_with_error_capture,
    signing_error_slot,
    take_signing_error,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TopicMessageData {
    pub consensus_time: OffsetDateTime,
    pub contents: Vec<u8>,
    pub sequence_number: u64,
}

pub struct GetTopicMessagesProps {
    pub topic_id: TopicId,
    pub from_time: Option<OffsetDateTime>,
    pub to_time: Option<OffsetDateTime>,
    pub limit: Option<usize>,
    pub max_idle_seconds: Option<u64>,
}

pub struct SubmitMessageResult {
    pub topic_id: String,
    pub sequence_number: u64,
}

pub struct HcsMessage;

impl HcsMessage {
    /// Submit a message to an HCS topic.
    /// Pass `submit_key_signer` for access-controlled topics.
    pub async fn submit(
        client: &Client,
        topic_id: TopicId,
        message: impl Into<Vec<u8>>,
        submit_key_signer: Option<Arc<dyn Signer>>,
    ) -> Result<SubmitMessageResult, DIDError> {
        let mut tx = TopicMessageSubmitTransaction::new();
        tx.topic_id(topic_id).message(message);
        let signing_errors = signing_error_slot();

        if let Some(signer) = submit_key_signer {
            let pk = public_key_from_signer(signer.as_ref())?;
            tx.sign_with(
                pk,
                sign_with_error_capture(Arc::clone(&signer), Arc::clone(&signing_errors)),
            );
        }

        let response = match tx.execute(client).await {
            Ok(response) => response,
            Err(e) => {
                take_signing_error(&signing_errors)?;
                return Err(DIDError::InternalError(format!("submit_execute_failed: {e}")));
            }
        };
        take_signing_error(&signing_errors)?;

        let receipt = response
            .get_receipt(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("submit_receipt_failed: {e}")))?;

        if receipt.status != hiero_sdk::Status::Success {
            return Err(DIDError::InternalError(format!(
                "Message submit failed: {:?}",
                receipt.status
            )));
        }

        Ok(SubmitMessageResult {
            topic_id: topic_id.to_string(),
            sequence_number: receipt.topic_sequence_number,
        })
    }

    /// Fetch historical topic messages via mirror node subscription stream.
    pub async fn get_topic_messages(
        client: &Client,
        props: GetTopicMessagesProps,
    ) -> Result<Vec<TopicMessageData>, DIDError> {
        Self::get_topic_messages_with_cache(client, props, "", None).await
    }

    /// Fetch historical topic messages via mirror node subscription stream with
    /// optional cache dedup+merge support.
    pub async fn get_topic_messages_with_cache(
        client: &Client,
        props: GetTopicMessagesProps,
        network_name: &str,
        cache: Option<&HcsCacheService>,
    ) -> Result<Vec<TopicMessageData>, DIDError> {
        let idle_timeout = Duration::from_secs(props.max_idle_seconds.unwrap_or(2));
        let topic_id_str = props.topic_id.to_string();
        let cached = if let Some(cache) = cache {
            cache.get_topic_messages(network_name, &topic_id_str).await
        } else {
            None
        };

        let mut query = TopicMessageQuery::new();
        query.topic_id(props.topic_id);

        if let Some(from) = props.from_time {
            query.start_time(from);
        }

        if let Some(to) = props.to_time {
            query.end_time(to);
        }

        if let Some(limit) = props.limit {
            query.limit(limit as u64);
        }

        let mut stream = query.subscribe(client);
        let mut fresh_results: Vec<TopicMessageData> = Vec::new();

        loop {
            match tokio::time::timeout(idle_timeout, stream.next()).await {
                Ok(None) => break,
                Ok(Some(Ok(msg))) => {
                    let consensus_time = msg.consensus_timestamp;
                    fresh_results.push(TopicMessageData {
                        consensus_time,
                        contents: msg.contents.to_vec(),
                        sequence_number: msg.sequence_number,
                    });
                    if let Some(to) = props.to_time {
                        if consensus_time >= to {
                            break;
                        }
                    }
                    if let Some(limit) = props.limit {
                        if fresh_results.len() >= limit {
                            break;
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let msg = e.to_string();
                    if msg.contains("NOT_FOUND") || msg.contains("5 NOT_FOUND") {
                        break;
                    }
                    return Err(DIDError::InternalError(format!(
                        "Error reading topic stream: {e}"
                    )));
                }
                Err(_) => break,
            }
        }

        let mut results = merge_dedup_topic_messages(cached.unwrap_or_default(), fresh_results);
        results.sort_by_key(|m| m.consensus_time);
        if let Some(limit) = props.limit {
            if results.len() > limit {
                results.truncate(limit);
            }
        }

        if let Some(cache) = cache {
            if props.from_time.is_none() && props.to_time.is_none() && props.limit.is_none() {
                cache.set_topic_messages(network_name, &topic_id_str, &results).await;
            }
        }

        Ok(results)
    }
}

fn merge_dedup_topic_messages(
    cached: Vec<TopicMessageData>,
    fresh: Vec<TopicMessageData>,
) -> Vec<TopicMessageData> {
    use std::collections::BTreeMap;

    let mut by_sequence: BTreeMap<u64, TopicMessageData> = BTreeMap::new();
    for msg in cached {
        by_sequence.insert(msg.sequence_number, msg);
    }
    for msg in fresh {
        by_sequence.insert(msg.sequence_number, msg);
    }
    by_sequence.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_dedup_replaces_cached_entry_with_fresh_same_sequence() {
        let cached = vec![
            TopicMessageData {
                consensus_time: OffsetDateTime::UNIX_EPOCH,
                contents: b"old".to_vec(),
                sequence_number: 1,
            },
            TopicMessageData {
                consensus_time: OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1),
                contents: b"keep".to_vec(),
                sequence_number: 2,
            },
        ];

        let fresh = vec![TopicMessageData {
            consensus_time: OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(2),
            contents: b"new".to_vec(),
            sequence_number: 1,
        }];

        let merged = merge_dedup_topic_messages(cached, fresh);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].sequence_number, 1);
        assert_eq!(merged[0].contents, b"new");
        assert_eq!(merged[1].sequence_number, 2);
    }
}
