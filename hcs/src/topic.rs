use hiero_did_core::DIDError;
use hiero_sdk::{
    Client, TopicCreateTransaction, TopicId, TopicMessageSubmitTransaction,
};

pub struct SubmitMessageResult {
    pub topic_id: String,
    pub sequence_number: Option<u64>,
}

pub struct HcsTopic;

impl HcsTopic {
    /// Create a new HCS topic and return its ID
    pub async fn create(client: &Client) -> Result<TopicId, DIDError> {
        let receipt = TopicCreateTransaction::new()
            .execute(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to create topic: {}", e)))?
            .get_receipt(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to get topic receipt: {}", e)))?;

        receipt
            .topic_id
            .ok_or_else(|| DIDError::InternalError("No topic ID in receipt".into()))
    }

    /// Create a new HCS topic with a memo
    pub async fn create_with_memo(client: &Client, memo: &str) -> Result<TopicId, DIDError> {
        let receipt = TopicCreateTransaction::new()
            .topic_memo(memo)
            .execute(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to create topic: {}", e)))?
            .get_receipt(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to get topic receipt: {}", e)))?;

        receipt
            .topic_id
            .ok_or_else(|| DIDError::InternalError("No topic ID in receipt".into()))
    }

    /// Submit a message to an HCS topic
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
            .map_err(|e| DIDError::InternalError(format!("Failed to submit message: {}", e)))?
            .get_receipt(client)
            .await
            .map_err(|e| DIDError::InternalError(format!("Failed to get message receipt: {}", e)))?;

        Ok(SubmitMessageResult {
            topic_id: format!("{}", topic_id),
            sequence_number: Some(receipt.topic_sequence_number),
        })
    }
}
