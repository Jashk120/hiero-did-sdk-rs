use hiero_did_core::DIDError;
use serde::Deserialize;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

const TESTNET_MIRROR: &str = "https://testnet.mirrornode.hedera.com";
const MAINNET_MIRROR: &str = "https://mainnet.mirrornode.hedera.com";

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

    /// Fetch all messages for a topic, paginating through all pages
    /// Returns decoded UTF-8 message strings in consensus order
    pub async fn get_topic_messages(&self, topic_id: &str) -> Result<Vec<String>, DIDError> {
        let mut messages = Vec::new();
        let mut url = Some(format!(
            "{}/api/v1/topics/{}/messages?order=asc&limit=100",
            self.base_url, topic_id
        ));

        while let Some(current_url) = url {
            let response = self.client
                .get(&current_url)
                .send()
                .await
                .map_err(|e| DIDError::InternalError(format!("Mirror node request failed: {}", e)))?;

            if !response.status().is_success() {
                return Err(DIDError::InternalError(format!(
                    "Mirror node returned status: {}",
                    response.status()
                )));
            }

            let body: MirrorTopicMessagesResponse = response
                .json()
                .await
                .map_err(|e| DIDError::InternalError(format!("Failed to parse mirror response: {}", e)))?;

            for msg in body.messages {
                let decoded = BASE64.decode(&msg.message)
                    .map_err(|e| DIDError::InternalError(format!("Failed to decode message: {}", e)))?;
                let text = String::from_utf8(decoded)
                    .map_err(|e| DIDError::InternalError(format!("Message is not valid UTF-8: {}", e)))?;
                messages.push(text);
            }

            // follow pagination
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
