use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;

use crate::hcs::{TopicInfo, TopicMessageData};

const DEFAULT_CACHE_MAX_SIZE: u64 = 1000;
const CACHE_TTL_SECS: u64 = 300;

#[derive(Clone)]
pub struct HcsCacheService {
    inner: Cache<String, Arc<Vec<u8>>>,
}

impl HcsCacheService {
    pub fn new(max_size: u64) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(max_size)
                .time_to_live(Duration::from_secs(CACHE_TTL_SECS))
                .build(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_CACHE_MAX_SIZE)
    }

    pub async fn get_topic_info(&self, network_name: &str, topic_id: &str) -> Option<TopicInfo> {
        let key = self.build_key(network_name, "info", topic_id);
        let bytes = self.inner.get(&key).await?;
        serde_json::from_slice(&bytes).ok()
    }

    pub async fn set_topic_info(&self, network_name: &str, topic_id: &str, info: &TopicInfo) {
        let key = self.build_key(network_name, "info", topic_id);
        if let Ok(bytes) = serde_json::to_vec(info) {
            self.inner.insert(key, Arc::new(bytes)).await;
        }
    }

    pub async fn remove_topic_info(&self, network_name: &str, topic_id: &str) {
        self.inner
            .remove(&self.build_key(network_name, "info", topic_id))
            .await;
        self.remove_topic_messages(network_name, topic_id).await;
    }

    pub async fn get_topic_messages(
        &self,
        network_name: &str,
        topic_id: &str,
    ) -> Option<Vec<TopicMessageData>> {
        let key = self.build_key(network_name, "messages", topic_id);
        let bytes = self.inner.get(&key).await?;
        serde_json::from_slice(&bytes).ok()
    }

    pub async fn set_topic_messages(
        &self,
        network_name: &str,
        topic_id: &str,
        messages: &[TopicMessageData],
    ) {
        let key = self.build_key(network_name, "messages", topic_id);
        if let Ok(bytes) = serde_json::to_vec(messages) {
            self.inner.insert(key, Arc::new(bytes)).await;
        }
        self.remove_topic_file(network_name, topic_id).await;
    }

    pub async fn remove_topic_messages(&self, network_name: &str, topic_id: &str) {
        self.inner
            .remove(&self.build_key(network_name, "messages", topic_id))
            .await;
        self.remove_topic_file(network_name, topic_id).await;
    }

    pub async fn get_topic_file(&self, network_name: &str, topic_id: &str) -> Option<Vec<u8>> {
        let key = self.build_key(network_name, "file", topic_id);
        let bytes = self.inner.get(&key).await?;
        Some(bytes.as_ref().clone())
    }

    pub async fn set_topic_file(&self, network_name: &str, topic_id: &str, file: &[u8]) {
        let key = self.build_key(network_name, "file", topic_id);
        self.inner.insert(key, Arc::new(file.to_vec())).await;
    }

    pub async fn remove_topic_file(&self, network_name: &str, topic_id: &str) {
        self.inner
            .remove(&self.build_key(network_name, "file", topic_id))
            .await;
    }

    fn build_key(&self, network_name: &str, target: &str, topic_id: &str) -> String {
        format!("{network_name}-{target}-{topic_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    fn sample_info(topic_id: &str) -> TopicInfo {
        TopicInfo {
            topic_id: topic_id.to_string(),
            topic_memo: "memo".to_string(),
            admin_key: None,
            submit_key: None,
            auto_renew_period_seconds: Some(7890000),
            auto_renew_account_id: None,
            expiration_time: None,
        }
    }

    fn sample_messages() -> Vec<TopicMessageData> {
        vec![
            TopicMessageData {
                consensus_time: OffsetDateTime::UNIX_EPOCH,
                contents: b"m1".to_vec(),
                sequence_number: 1,
            },
            TopicMessageData {
                consensus_time: OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1),
                contents: b"m2".to_vec(),
                sequence_number: 2,
            },
        ]
    }

    #[tokio::test]
    async fn topic_info_is_namespaced_by_network() {
        let cache = HcsCacheService::with_defaults();
        let info = sample_info("0.0.1001");
        cache.set_topic_info("testnet", "0.0.1001", &info).await;

        let hit = cache.get_topic_info("testnet", "0.0.1001").await;
        let miss = cache.get_topic_info("mainnet", "0.0.1001").await;

        assert!(hit.is_some());
        assert!(miss.is_none());
    }

    #[tokio::test]
    async fn remove_topic_info_cascades_messages_and_file() {
        let cache = HcsCacheService::with_defaults();
        let info = sample_info("0.0.1002");
        let messages = sample_messages();
        cache.set_topic_info("testnet", "0.0.1002", &info).await;
        cache
            .set_topic_messages("testnet", "0.0.1002", &messages)
            .await;
        cache
            .set_topic_file("testnet", "0.0.1002", b"payload")
            .await;

        cache.remove_topic_info("testnet", "0.0.1002").await;

        assert!(cache.get_topic_info("testnet", "0.0.1002").await.is_none());
        assert!(
            cache
                .get_topic_messages("testnet", "0.0.1002")
                .await
                .is_none()
        );
        assert!(cache.get_topic_file("testnet", "0.0.1002").await.is_none());
    }

    #[tokio::test]
    async fn set_topic_messages_invalidates_file_cache() {
        let cache = HcsCacheService::with_defaults();
        cache
            .set_topic_file("testnet", "0.0.1003", b"old-file")
            .await;
        cache
            .set_topic_messages("testnet", "0.0.1003", &sample_messages())
            .await;

        assert!(cache.get_topic_file("testnet", "0.0.1003").await.is_none());
    }
}
