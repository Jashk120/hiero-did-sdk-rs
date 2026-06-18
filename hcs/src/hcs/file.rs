use std::str::FromStr;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hiero_did_core::DIDError;
use hiero_sdk::{
    Client,
    TopicId,
};
use sha2::{
    Digest,
    Sha256,
};
use time::OffsetDateTime;

use crate::cache::HcsCacheService;
use crate::hcs::message::{
    GetTopicMessagesProps,
    HcsMessage,
};
use crate::hcs::topic::{
    CreateTopicProps,
    HcsTopic,
};
use crate::shared::wait_for_changes;

const HCS1_MEMO_PATTERN: &str = ":zstd:base64";
const BASE64_JSON_CONTENT_PREFIX: &str = "data:application/json;base64,";
const MAX_CHUNK_CONTENT_SIZE: usize = 960;

// ── Public types ──────────────────────────────────────────────────────────────

pub struct SubmitFileProps {
    pub payload: Vec<u8>,
    pub submit_key_signer: Arc<dyn hiero_did_core::Signer>,
    pub wait_for_visibility: bool,
    pub wait_timeout_ms: Option<u64>,
}

pub struct ResolveFileProps {
    pub topic_id: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ChunkMessage {
    o: usize,
    c: String,
}

// ── HcsFileService ────────────────────────────────────────────────────────────

pub struct HcsFileService<'a> {
    client: &'a Client,
    network_name: String,
    cache: Option<HcsCacheService>,
}

impl<'a> HcsFileService<'a> {
    pub fn new(
        client: &'a Client,
        network_name: impl Into<String>,
        cache: Option<HcsCacheService>,
    ) -> Self {
        Self { client, network_name: network_name.into(), cache }
    }

    pub async fn submit_file(&self, props: SubmitFileProps) -> Result<String, DIDError> {
        let payload_hash = sha256_hex(&props.payload);
        let memo = format!("{payload_hash}{HCS1_MEMO_PATTERN}");

        let topic_id = HcsTopic::create_with_props(
            self.client,
            CreateTopicProps {
                topic_memo: Some(memo),
                submit_key_signer: Some(Arc::clone(&props.submit_key_signer)),
                ..Default::default()
            },
        )
        .await?;

        let chunks = self.build_chunks_from_payload(&props.payload)?;

        for (order_index, content) in &chunks {
            let message =
                serde_json::to_string(&ChunkMessage { o: *order_index, c: content.clone() })
                    .map_err(|e| DIDError::SerializationError(e.to_string()))?;

            HcsMessage::submit(
                self.client,
                topic_id,
                message.into_bytes(),
                Some(Arc::clone(&props.submit_key_signer)),
            )
            .await?;
        }

        let topic_id_str = topic_id.to_string();

        if props.wait_for_visibility {
            let client = self.client.clone();
            let payload_clone = props.payload.clone();
            let tid = topic_id_str.clone();

            wait_for_changes(
                move || {
                    let c = client.clone();
                    let p = payload_clone.clone();
                    let t = tid.clone();
                    async move {
                        let svc = HcsFileService::new(&c, "", None);
                        svc.resolve_file_without_cache(&ResolveFileProps { topic_id: t })
                            .await
                            .map(|resolved| resolved == p)
                            .map_err(|e| e.to_string())
                    }
                },
                |matched: &bool| *matched,
                props.wait_timeout_ms,
                None,
            )
            .await
            .map_err(|e| DIDError::InternalError(e))?;
        }

        Ok(topic_id_str)
    }

    pub async fn resolve_file(&self, props: &ResolveFileProps) -> Result<Vec<u8>, DIDError> {
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_topic_file(&self.network_name, &props.topic_id).await {
                return Ok(cached);
            }
        }

        let payload = self.resolve_file_without_cache(props).await?;

        if let Some(cache) = &self.cache {
            cache.set_topic_file(&self.network_name, &props.topic_id, &payload).await;
        }

        Ok(payload)
    }

    async fn resolve_file_without_cache(
        &self,
        props: &ResolveFileProps,
    ) -> Result<Vec<u8>, DIDError> {
        let topic_id = TopicId::from_str(&props.topic_id)
            .map_err(|e| DIDError::InvalidArgument(format!("Invalid topic ID: {e}")))?;

        let info = HcsTopic::get_info(self.client, topic_id).await?;

        if !is_valid_hcs1_memo(&info.topic_memo) {
            return Err(DIDError::InvalidArgument(format!(
                "Topic {} memo is not HCS-1 compliant",
                props.topic_id
            )));
        }

        if info.admin_key.is_some() {
            return Err(DIDError::InvalidArgument(format!(
                "HCS file topic {} must not have an admin key",
                props.topic_id
            )));
        }

        let messages = HcsMessage::get_topic_messages(
            self.client,
            GetTopicMessagesProps {
                topic_id,
                from_time: Some(OffsetDateTime::UNIX_EPOCH),
                to_time: None,
                limit: None,
                max_idle_seconds: Some(2),
            },
        )
        .await?;

        let chunks: Vec<ChunkMessage> = messages
            .iter()
            .filter_map(|m| {
                let text = String::from_utf8(m.contents.clone()).ok()?;
                serde_json::from_str(&text).ok()
            })
            .collect();

        let payload = self.build_payload_from_chunks(chunks)?;

        let checksum = sha256_hex(&payload);
        if !is_valid_hcs1_checksum(&info.topic_memo, &checksum) {
            return Err(DIDError::InvalidArgument("HCS-1 file checksum mismatch".into()));
        }

        Ok(payload)
    }

    fn build_chunks_from_payload(&self, payload: &[u8]) -> Result<Vec<(usize, String)>, DIDError> {
        let compressed = zstd::encode_all(payload, 0)
            .map_err(|e| DIDError::InternalError(format!("zstd compress failed: {e}")))?;

        let b64 = BASE64.encode(&compressed);
        let content = format!("{BASE64_JSON_CONTENT_PREFIX}{b64}");
        let encoded = content.as_bytes();

        let chunks = encoded
            .chunks(MAX_CHUNK_CONTENT_SIZE)
            .enumerate()
            .map(|(i, chunk)| (i, String::from_utf8_lossy(chunk).to_string()))
            .collect();

        Ok(chunks)
    }

    fn build_payload_from_chunks(
        &self,
        mut chunks: Vec<ChunkMessage>,
    ) -> Result<Vec<u8>, DIDError> {
        chunks.sort_by_key(|c| c.o);

        let content: String = chunks.into_iter().map(|c| c.c).collect();

        let b64_data = content.strip_prefix(BASE64_JSON_CONTENT_PREFIX).unwrap_or(&content);

        let compressed = BASE64
            .decode(b64_data)
            .map_err(|e| DIDError::InvalidArgument(format!("base64 decode failed: {e}")))?;

        zstd::decode_all(compressed.as_slice())
            .map_err(|e| DIDError::InternalError(format!("zstd decompress failed: {e}")))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn is_valid_hcs1_memo(memo: &str) -> bool {
    if memo.len() != 64 + HCS1_MEMO_PATTERN.len() {
        return false;
    }
    let (hash_part, suffix) = memo.split_at(64);
    suffix == HCS1_MEMO_PATTERN && hash_part.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_hcs1_checksum(memo: &str, checksum: &str) -> bool {
    memo.split(':').next().map(|h| h == checksum).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_service() -> HcsFileService<'static> {
        let client = Box::leak(Box::new(Client::for_testnet()));
        HcsFileService::new(client, "testnet", None)
    }

    #[tokio::test]
    async fn chunk_roundtrip_rebuilds_original_payload() {
        let svc = file_service();
        let payload = "hello-hedera-".repeat(400).into_bytes();

        let chunks = svc.build_chunks_from_payload(&payload).expect("chunking should succeed");
        assert!(!chunks.is_empty());

        let chunk_messages: Vec<ChunkMessage> =
            chunks.iter().map(|(o, c)| ChunkMessage { o: *o, c: c.clone() }).collect();

        let rebuilt =
            svc.build_payload_from_chunks(chunk_messages).expect("rebuild should succeed");
        assert_eq!(rebuilt, payload);
    }

    #[tokio::test]
    async fn build_payload_from_unsorted_chunks_sorts_by_order_index() {
        let svc = file_service();
        let payload = b"order-check-payload".repeat(100);

        let mut chunks = svc.build_chunks_from_payload(&payload).expect("chunking should succeed");
        chunks.reverse();

        let chunk_messages: Vec<ChunkMessage> =
            chunks.iter().map(|(o, c)| ChunkMessage { o: *o, c: c.clone() }).collect();

        let rebuilt =
            svc.build_payload_from_chunks(chunk_messages).expect("rebuild should succeed");
        assert_eq!(rebuilt, payload);
    }

    #[test]
    fn hcs1_memo_validation_accepts_valid_and_rejects_invalid() {
        let checksum = "a".repeat(64);
        let valid = format!("{checksum}{HCS1_MEMO_PATTERN}");
        assert!(is_valid_hcs1_memo(&valid));
        assert!(!is_valid_hcs1_memo("short"));
        assert!(!is_valid_hcs1_memo(&format!("{checksum}:not-zstd")));
        assert!(!is_valid_hcs1_memo(&format!("{}{}", "g".repeat(64), HCS1_MEMO_PATTERN)));
    }

    #[test]
    fn hcs1_checksum_validation_uses_prefix_hash() {
        let payload = b"checksum-payload";
        let checksum = sha256_hex(payload);
        let memo = format!("{checksum}{HCS1_MEMO_PATTERN}");
        assert!(is_valid_hcs1_checksum(&memo, &checksum));
        assert!(!is_valid_hcs1_checksum(&memo, "deadbeef"));
    }
}
