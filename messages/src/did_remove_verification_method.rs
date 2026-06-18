use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hiero_did_core::DIDError;
use serde_json;

use crate::envelope::{
    HcsEnvelope,
    HcsMessage,
};
use crate::events::{
    DIDRemoveVerificationMethodEvent,
    DIDRemoveVerificationMethodEventData,
};

#[derive(Debug, Clone)]
pub struct DIDRemoveVerificationMethodMessage {
    pub did: String,
    /// Full fragment id to remove, e.g. `did:hedera:testnet:...#key-1`
    pub id: String,
    pub timestamp: String,
}

impl DIDRemoveVerificationMethodMessage {
    pub fn new(did: String, id: String) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Self { did, id, timestamp }
    }

    pub fn to_hcs_message(&self) -> Result<HcsMessage, DIDError> {
        let event = DIDRemoveVerificationMethodEvent {
            verification_method: DIDRemoveVerificationMethodEventData { id: self.id.clone() },
        };
        let event_json = serde_json::to_string(&event)
            .map_err(|e| DIDError::SerializationError(e.to_string()))?;
        let event_b64 = BASE64.encode(event_json.as_bytes());
        Ok(HcsMessage {
            timestamp: self.timestamp.clone(),
            operation: "update".to_string(),
            did: self.did.clone(),
            event: Some(event_b64),
        })
    }

    pub fn to_payload(&self, signature: &[u8]) -> Result<String, DIDError> {
        let message = self.to_hcs_message()?;
        let envelope = HcsEnvelope { message, signature: BASE64.encode(signature) };
        serde_json::to_string(&envelope).map_err(|e| DIDError::SerializationError(e.to_string()))
    }

    pub fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        let message = self.to_hcs_message()?;
        serde_json::to_vec(&message).map_err(|e| DIDError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::DIDRemoveVerificationMethodMessage;
    use crate::envelope::HcsEnvelope;

    #[test]
    fn builds_remove_vm_payload() {
        let msg = DIDRemoveVerificationMethodMessage::new(
            "did:hedera:testnet:abc_0.0.1".to_string(),
            "did:hedera:testnet:abc_0.0.1#key-1".to_string(),
        );
        let payload = msg.to_payload(&[2u8; 64]).expect("payload");
        let envelope: HcsEnvelope = serde_json::from_str(&payload).expect("valid envelope");
        assert_eq!(envelope.message.operation, "update");
        assert_eq!(envelope.message.did, "did:hedera:testnet:abc_0.0.1");
    }
}
