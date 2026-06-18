use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hiero_did_core::DIDError;
use serde::Deserialize;
use crate::topic_reader::TopicReader;
use async_trait::async_trait;

const TESTNET_MIRROR: &str = "https://testnet.mirrornode.hedera.com";
const MAINNET_MIRROR: &str = "https://mainnet.mirrornode.hedera.com";
const LOCAL_MIRROR: &str = "http://localhost:38081";

#[derive(Debug, Deserialize)]
pub struct MirrorTopicMessage {
    /// base64-encoded message content
    pub message: String,
    pub consensus_timestamp: String,
}

#[derive(Debug, Deserialize)]
struct MirrorTopicMessagesResponse {
    messages: Vec<MirrorTopicMessage>,
    #[serde(default)]
    links: MirrorLinks,
}

#[derive(Debug, Deserialize, Default)]
struct MirrorLinks {
    next: Option<String>,
}

pub struct MirrorNodeClient {
    base_url: String,
    client: reqwest::Client,
}

impl MirrorNodeClient {
    pub fn for_testnet() -> Self {
        Self {
            base_url: TESTNET_MIRROR.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn for_mainnet() -> Self {
        Self {
            base_url: MAINNET_MIRROR.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn for_local() -> Self {
        Self {
            base_url: LOCAL_MIRROR.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Self {
        let base_url = std::env::var("MIRROR_BASE_URL").unwrap_or_else(|_| {
            match std::env::var("HEDERA_NETWORK").as_deref() {
                Ok("mainnet") => MAINNET_MIRROR.to_string(),
                Ok("local") | Ok("local-node") | Ok("localhost") => LOCAL_MIRROR.to_string(),
                _ => TESTNET_MIRROR.to_string(),
            }
        });

        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// Poll until at least one message appears on the topic, or timeout.
    /// Good for create and deactivate — single message expected.
    pub async fn wait_for_mirror(
        &self,
        topic_id: &str,
        timeout_secs: u64,
    ) -> Result<Vec<String>, DIDError> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);

        loop {
            let messages = self.get_topic_messages(topic_id).await?;
            if !messages.is_empty() {
                return Ok(messages);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(DIDError::InternalError(format!(
                    "Timed out waiting for mirror node to index topic {}",
                    topic_id
                )));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    /// Poll until no new messages arrive for `stable_window_ms` milliseconds.
    /// Good for update flows where multiple messages are submitted — avoids
    /// resolving before all messages are indexed.
    pub async fn wait_for_mirror_stable(
        &self,
        topic_id: &str,
        stable_window_ms: u64,
        timeout_secs: u64,
    ) -> Result<Vec<String>, DIDError> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);
        let mut last_count = 0;
        let mut stable_since: Option<tokio::time::Instant> = None;

        loop {
            let messages = self.get_topic_messages(topic_id).await?;

            if messages.len() != last_count {
                last_count = messages.len();
                stable_since = Some(tokio::time::Instant::now());
            } else if let Some(since) = stable_since {
                if since.elapsed().as_millis() as u64 >= stable_window_ms {
                    return Ok(messages);
                }
            } else {
                stable_since = Some(tokio::time::Instant::now());
            }

            if tokio::time::Instant::now() >= deadline {
                return Err(DIDError::InternalError(format!(
                    "Timed out waiting for stable state on topic {}",
                    topic_id
                )));
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
    }

    /// Fetch all messages for a topic, paginating through all pages.
    /// Returns decoded UTF-8 message strings in consensus order.
    pub async fn get_topic_messages(&self, topic_id: &str) -> Result<Vec<String>, DIDError> {
        let mut messages = Vec::new();
        let mut url = Some(format!(
            "{}/api/v1/topics/{}/messages?order=asc&limit=100",
            self.base_url, topic_id
        ));

        while let Some(current_url) = url {
            let response = self.client.get(&current_url).send().await.map_err(|e| {
                DIDError::InternalError(format!("Mirror node request failed: {}", e))
            })?;

            if !response.status().is_success() {
                return Err(DIDError::InternalError(format!(
                    "Mirror node returned status: {}",
                    response.status()
                )));
            }

            let body: MirrorTopicMessagesResponse = response.json().await.map_err(|e| {
                DIDError::InternalError(format!("Failed to parse mirror response: {}", e))
            })?;

            for msg in body.messages {
                let decoded = BASE64.decode(&msg.message).map_err(|e| {
                    DIDError::InternalError(format!("Failed to decode message: {}", e))
                })?;
                let text = String::from_utf8(decoded).map_err(|e| {
                    DIDError::InternalError(format!("Message is not valid UTF-8: {}", e))
                })?;
                messages.push(text);
            }

            url = body.links.next.map(|next| {
                if next.starts_with("http") {
                    next
                } else {
                    format!("{}{}", self.base_url, next)
                }
            });
        }

        Ok(messages)
    }
}

#[async_trait]
impl TopicReader for MirrorNodeClient {
    async fn get_topic_messages(&self, topic_id: &str) -> Result<Vec<String>, DIDError> {
        MirrorNodeClient::get_topic_messages(self, topic_id).await
    }
}