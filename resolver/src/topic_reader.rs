use async_trait::async_trait;
use hiero_did_core::DIDError;

/// Abstraction over how topic messages get fetched, so resolver code isn't
/// hardcoded to one transport. Implemented by MirrorNodeClient (REST) and
/// GrpcTopicReader (gRPC, via hiero-did-hcs).
#[async_trait]
pub trait TopicReader: Send + Sync {
    async fn get_topic_messages(&self, topic_id: &str) -> Result<Vec<String>, DIDError>;
}