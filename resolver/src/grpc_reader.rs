use std::str::FromStr;

use async_trait::async_trait;
use hiero_did_core::DIDError;
use hiero_did_hcs::{
    GetTopicMessagesProps,
    HcsClient,
    HcsMessage,
};
use time::OffsetDateTime;

use crate::topic_reader::TopicReader;

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
        Self { client: HcsClient::for_testnet() }
    }

    pub fn for_mainnet() -> Self {
        Self { client: HcsClient::for_mainnet() }
    }

    /// Create a reader from an already-configured [`HcsClient`] pointed at testnet.
    ///
    /// Use this when you need to supply an operator (account id + key) for
    /// access-controlled topics or authenticated gRPC subscriptions.
    pub fn for_testnet_with_client(client: HcsClient) -> Self {
        Self { client }
    }

    /// Create a reader from an already-configured [`HcsClient`] pointed at mainnet.
    ///
    /// Use this when you need to supply an operator (account id + key) for
    /// access-controlled topics or authenticated gRPC subscriptions.
    pub fn for_mainnet_with_client(client: HcsClient) -> Self {
        Self { client }
    }

    /// Create a reader from an already-configured [`HcsClient`] pointed at a
    /// local Hedera node (hedera-local-node).
    ///
    /// Use [`HcsClient::for_local_node_with_operator`] to build the client, or
    /// supply any pre-configured client whose mirror network is set to
    /// `127.0.0.1:38081` (or `HEDERA_MIRROR_NODE_ADDRESS`).
    pub fn for_local_node_with_client(client: HcsClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl TopicReader for GrpcTopicReader {
    async fn get_topic_messages(&self, topic_id: &str) -> Result<Vec<String>, DIDError> {
        let topic_id = hiero_sdk::TopicId::from_str(topic_id)
            .map_err(|e| DIDError::InvalidArgument(format!("Invalid topic id: {e}")))?;

        let props = GetTopicMessagesProps {
            topic_id,
            // Setting from_time to UNIX_EPOCH tells the gRPC mirror to stream
            // from the very first message on this topic.  Without it,
            // consensus_start_time is omitted from the proto request and the
            // mirror defaults to "start from now", returning zero historical
            // messages before the idle timeout fires.
            from_time: Some(OffsetDateTime::UNIX_EPOCH),
            to_time: None,
            limit: None,
            max_idle_seconds: None,
        };

        let messages = HcsMessage::get_topic_messages(&self.client.inner, props).await?;

        messages
            .into_iter()
            .map(|m| {
                String::from_utf8(m.contents).map_err(|e| {
                    DIDError::InternalError(format!("Message is not valid UTF-8: {e}"))
                })
            })
            .collect()
    }
}