use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hiero_did_core::DIDError;
use serde_json;
use crate::envelope::{HcsEnvelope, HcsMessage};
use crate::events::{DIDAddVerificationMethodEvent, DIDAddVerificationMethodEventData};
 
pub struct DIDAddVerificationMethodMessage {
    pub did: String,
    /// Full fragment id, e.g. `did:hedera:testnet:...#key-1`
    pub id: String,
    /// e.g. "verificationMethod", "authentication", "assertionMethod"
    pub property: String,
    pub controller: String,
    pub public_key_multibase: String,
    pub timestamp: String,
}
 
impl DIDAddVerificationMethodMessage {
    pub fn new(
        did: String,
        id: String,
        property: String,
        controller: String,
        public_key_multibase: String,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Self { did, id, property, controller, public_key_multibase, timestamp }
    }
 
    pub fn to_hcs_message(&self) -> Result<HcsMessage, DIDError> {
        let event = DIDAddVerificationMethodEvent {
            verification_method: DIDAddVerificationMethodEventData {
                id: self.id.clone(),
                key_type: "Ed25519VerificationKey2020".to_string(),
                controller: self.controller.clone(),
                public_key_multibase: self.public_key_multibase.clone(),
                relationship_type: self.property.clone(),
            },
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
        let envelope = HcsEnvelope {
            message,
            signature: BASE64.encode(signature),
        };
        serde_json::to_string(&envelope)
            .map_err(|e| DIDError::SerializationError(e.to_string()))
    }
 
    pub fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        let message = self.to_hcs_message()?;
        serde_json::to_vec(&message)
            .map_err(|e| DIDError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::DIDAddVerificationMethodMessage;
    use crate::envelope::HcsEnvelope;

    #[test]
    fn builds_update_payload() {
        let msg = DIDAddVerificationMethodMessage::new(
            "did:hedera:testnet:abc_0.0.1".to_string(),
            "did:hedera:testnet:abc_0.0.1#key-1".to_string(),
            "authentication".to_string(),
            "did:hedera:testnet:abc_0.0.1".to_string(),
            "zKey".to_string(),
        );
        let bytes = msg.message_bytes().expect("message bytes");
        assert!(!bytes.is_empty());
        let payload = msg.to_payload(&[1u8; 64]).expect("payload");
        let envelope: HcsEnvelope = serde_json::from_str(&payload).expect("valid envelope");
        assert_eq!(envelope.message.operation, "update");
        assert_eq!(envelope.message.did, "did:hedera:testnet:abc_0.0.1");
    }
}
