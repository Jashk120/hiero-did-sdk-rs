use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hiero_did_core::{DIDError, HederaDid, KeysUtility};
use hiero_did_lifecycle::LifecycleMessage;
use serde_json;

use crate::envelope::{HcsEnvelope, HcsMessage};
use crate::events::{DIDOwnerEvent, DIDOwnerEventData};

pub struct DIDOwnerMessage {
    pub did: HederaDid,
    /// Raw 32-byte ed25519 public key
    pub public_key_bytes: Vec<u8>,
    pub timestamp: String,
    pub controller: Option<String>,
    pub signature: Option<Vec<u8>>,
}

impl DIDOwnerMessage {
    pub fn new(
        did: HederaDid,
        public_key_bytes: Vec<u8>,
        controller: Option<String>,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Self {
            did,
            public_key_bytes,
            timestamp,
            controller,
            signature: None,
        }
    }

    /// Build the HcsMessage — this is what gets signed
    pub fn to_hcs_message(&self) -> Result<HcsMessage, DIDError> {
        let did_string = self.did.to_string();
        let controller = self.controller.clone().unwrap_or_else(|| did_string.clone());

        let event = DIDOwnerEvent {
            did_owner: DIDOwnerEventData {
                id: did_string.clone(),
                key_type: "Ed25519VerificationKey2020".to_string(),
                controller,
                public_key_multibase: KeysUtility::from_bytes(self.public_key_bytes.clone())
                    .to_multibase(),
            },
        };

        let event_json = serde_json::to_string(&event)
            .map_err(|e| DIDError::SerializationError(e.to_string()))?;
        let event_b64 = BASE64.encode(event_json.as_bytes());

        Ok(HcsMessage {
            timestamp: self.timestamp.clone(),
            operation: "create".to_string(),
            did: did_string,
            event: Some(event_b64),
        })
    }

    /// Build the signed envelope ready for HCS submission
    pub fn to_payload(&self, signature: &[u8]) -> Result<String, DIDError> {
        let message = self.to_hcs_message()?;
        let envelope = HcsEnvelope {
            message,
            signature: BASE64.encode(signature),
        };
        serde_json::to_string(&envelope)
            .map_err(|e| DIDError::SerializationError(e.to_string()))
    }

    /// The bytes that must be signed — JSON.stringify(message)
    pub fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        let message = self.to_hcs_message()?;
        serde_json::to_vec(&message)
            .map_err(|e| DIDError::SerializationError(e.to_string()))
    }
}

impl LifecycleMessage for DIDOwnerMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        self.message_bytes()
    }

    fn set_signature(&mut self, signature: Vec<u8>) -> Result<(), DIDError> {
        self.signature = Some(signature);
        Ok(())
    }
}