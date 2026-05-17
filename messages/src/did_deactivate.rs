use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hiero_did_core::{DIDError, HederaDid};
use serde_json;
use crate::envelope::{HcsEnvelope, HcsMessage};
use crate::events::{DIDDeactivateEvent, DIDDeactivateEventData};
 
pub struct DIDDeactivateMessage {
    pub did: HederaDid,
    pub timestamp: String,
}
 
impl DIDDeactivateMessage {
    pub fn new(did: HederaDid) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Self { did, timestamp }
    }
 
    pub fn to_hcs_message(&self) -> Result<HcsMessage, DIDError> {
        let did_string = self.did.to_string();
        let event = DIDDeactivateEvent {
            did_owner: DIDDeactivateEventData {
                id: did_string.clone(),
            },
        };
        let event_json = serde_json::to_string(&event)
            .map_err(|e| DIDError::SerializationError(e.to_string()))?;
        let event_b64 = BASE64.encode(event_json.as_bytes());
        Ok(HcsMessage {
            timestamp: self.timestamp.clone(),
            operation: "delete".to_string(),
            did: did_string,
            event: event_b64,
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
    use super::DIDDeactivateMessage;
    use crate::envelope::HcsEnvelope;
    use hiero_did_core::{did::Network, HederaDid};

    #[test]
    fn builds_deactivate_payload() {
        let did = HederaDid::new(Network::Testnet, "abc".to_string(), "0.0.1".to_string());
        let msg = DIDDeactivateMessage::new(did);
        let payload = msg.to_payload(&[5u8; 64]).expect("payload");
        let envelope: HcsEnvelope = serde_json::from_str(&payload).expect("valid envelope");
        assert_eq!(envelope.message.operation, "delete");
    }
}
