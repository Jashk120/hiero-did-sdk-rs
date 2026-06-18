use crate::topic_reader::TopicReader;
use async_trait::async_trait;
use hiero_did_core::DIDError;
use hiero_did_hcs::{GetTopicMessagesProps, HcsClient, HcsMessage};
use std::str::FromStr;

/// Topic reader backed by the gRPC mirror-node subscription stream
/// (hiero_sdk::TopicMessageQuery, via hiero-did-hcs::HcsMessage).
///
/// Use when REST mirror-node access is unavailable or undesired, e.g.
/// in environments with gRPC connectivity but restricted outbound HTTP,
/// or where you want stronger ordering/idle-timeout guarantees than
/// REST pagination naturally provides.
pub struct GrpcTopicReader {
    client: HcsClient,
}

impl GrpcTopicReader {
    pub fn for_testnet() -> Self {
        Self {
            client: HcsClient::for_testnet(),
        }
    }

    pub fn for_mainnet() -> Self {
        Self {
            client: HcsClient::for_mainnet(),
        }
    }
}

#[async_trait]
impl TopicReader for GrpcTopicReader {
    async fn get_topic_messages(&self, topic_id: &str) -> Result<Vec<String>, DIDError> {
        let topic_id = hiero_sdk::TopicId::from_str(topic_id)
            .map_err(|e| DIDError::InvalidArgument(format!("Invalid topic id: {e}")))?;

        let props = GetTopicMessagesProps {
            topic_id,
            from_time: None,
            to_time: None,
            limit: None,
            max_idle_seconds: None,
        };

        let messages = HcsMessage::get_topic_messages(&self.client.inner, props).await?;

        messages
            .into_iter()
            .map(|m| {
                String::from_utf8(m.contents)
                    .map_err(|e| DIDError::InternalError(format!("Message is not valid UTF-8: {e}")))
            })
            .collect()
    }
}