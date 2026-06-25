use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use hiero_did_core::DIDError;
use hiero_did_hcs::{GetTopicMessagesProps, HederaHcsService};

use crate::topic_reader::TopicReader;

/// Topic reader backed by [`HederaHcsService`], which uses the gRPC
/// `TopicMessageQuery` stream with an optional moka in-process cache
/// ([`hiero_did_hcs::HcsCacheService`]).
///
/// This is the preferred reader when you already have an `HederaHcsService`
/// in scope (e.g. from the registrar pipeline), because it reuses the shared
/// cache — resolved topics don't hit the network again until the cache entry
/// expires.
///
/// # Example
/// ```rust,no_run
/// use std::sync::Arc;
/// use hiero_did_hcs::{HederaClientService, HederaHcsService};
/// use hiero_did_resolver::{HcsTopicReader, TopicReader};
///
/// let client_service = HederaClientService::for_testnet();
/// let hcs_service = Arc::new(HederaHcsService::new(client_service, None));
/// let reader = HcsTopicReader::new(hcs_service, Some("testnet".to_string()));
/// ```
pub struct HcsTopicReader {
    service: Arc<HederaHcsService>,
    /// Network name forwarded to [`HederaHcsService::get_topic_messages`].
    /// `None` means the service's default network.
    network_name: Option<String>,
}

impl HcsTopicReader {
    pub fn new(service: Arc<HederaHcsService>, network_name: Option<String>) -> Self {
        Self { service, network_name }
    }

    /// Convenience constructor for testnet with no cache.
    pub fn for_testnet(service: Arc<HederaHcsService>) -> Self {
        Self::new(service, Some("testnet".to_string()))
    }

    /// Convenience constructor for mainnet with no cache.
    pub fn for_mainnet(service: Arc<HederaHcsService>) -> Self {
        Self::new(service, Some("mainnet".to_string()))
    }
}

#[async_trait]
impl TopicReader for HcsTopicReader {
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

        let messages = self
            .service
            .get_topic_messages(self.network_name.as_deref(), props)
            .await?;

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