use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hiero_did_core::{
    DIDError, DIDDocument, DIDDocumentMetadata, DIDResolution, DIDResolutionMetadata,
    KeyCapabilityMethod, KeysUtility, Service, VerificationMethod, VerificationMethodMultibase,
    did::{DID_ROOT_KEY_ID, HederaDid},
};
use hiero_did_messages::envelope::HcsEnvelope;
use hiero_did_messages::events::{DIDOwnerEvent, DIDEvent};
use hiero_did_signer::InternalVerifier;
use std::collections::HashMap;
use serde_json;

pub struct DidDocumentBuilder {
    messages: Vec<String>,
}

impl DidDocumentBuilder {
    pub fn from(messages: Vec<String>) -> Self {
        Self { messages }
    }

    pub async fn resolve(&self, did: &HederaDid) -> Result<DIDResolution, DIDError> {
        let did_string = did.to_string();

        let mut verifier: Option<InternalVerifier> = None;
        let mut verification_methods: HashMap<String, VerificationMethod> = HashMap::new();
        let services: HashMap<String, Service> = HashMap::new();
        let mut authentication: HashMap<String, KeyCapabilityMethod> = HashMap::new();
        let mut assertion_method: HashMap<String, KeyCapabilityMethod> = HashMap::new();
        let mut controller: Option<String> = None;
        let mut created_at: Option<String> = None;
        let mut updated_at: Option<String> = None;
        let mut deactivated = false;
        let mut exists = false;

        for raw in &self.messages {
            // parse envelope
            let envelope: HcsEnvelope = match serde_json::from_str(raw) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // filter to this DID only
            if envelope.message.did != did_string {
                continue;
            }

            let message = &envelope.message;

            // handle delete
            if message.operation == "delete" {
                if let Some(ref v) = verifier {
                    let msg_bytes = serde_json::to_vec(message)
                        .map_err(|e| DIDError::SerializationError(e.to_string()))?;
                    let sig_bytes = BASE64.decode(&envelope.signature)
                        .map_err(|_| DIDError::InvalidSignature("Bad base64 signature".into()))?;
                    if v.verify(&msg_bytes, &sig_bytes)? {
                        deactivated = true;
                        break;
                    }
                }
                continue;
            }

            // decode event
            let event_json = match BASE64.decode(&message.event) {
                Ok(b) => b,
                Err(_) => continue,
            };
            let event_str = match String::from_utf8(event_json) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let event: DIDEvent = match serde_json::from_str(&event_str) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // for DIDOwner: extract public key to build verifier before sig check
            if let DIDEvent::Owner(ref owner_event) = event {
                let key_bytes = KeysUtility::from_multibase(
                    &owner_event.did_owner.public_key_multibase
                )?.to_bytes().to_vec();

                verifier = Some(InternalVerifier::from_bytes(&key_bytes)?);
            }

            // verify signature
            let Some(ref v) = verifier else { continue };
            let msg_bytes = serde_json::to_vec(message)
                .map_err(|e| DIDError::SerializationError(e.to_string()))?;
            let sig_bytes = match BASE64.decode(&envelope.signature) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if !v.verify(&msg_bytes, &sig_bytes)? {
                continue;
            }

            // apply event
            match event {
                DIDEvent::Owner(owner_event) => {
                    exists = true;
                    apply_did_owner(
                        &owner_event,
                        &did_string,
                        &mut verification_methods,
                        &mut controller,
                    );
                    if created_at.is_none() {
                        created_at = Some(message.timestamp.clone());
                    }
                }
            }

            updated_at = Some(message.timestamp.clone());
        }

        if !exists {
            return Err(DIDError::NotFound(format!("DID document not found: {}", did_string)));
        }

        // build final document
        let root_key_id = did.root_key_id();

        // inject root key into authentication and assertionMethod if not present
        if !authentication.contains_key(&root_key_id) {
            authentication.insert(
                root_key_id.clone(),
                KeyCapabilityMethod::Reference(root_key_id.clone()),
            );
        }
        if !assertion_method.contains_key(&root_key_id) {
            assertion_method.insert(
                root_key_id.clone(),
                KeyCapabilityMethod::Reference(root_key_id.clone()),
            );
        }

        let did_document = DIDDocument {
            id: did_string.clone(),
            controller: controller.unwrap_or_else(|| did_string.clone()),
            verification_method: verification_methods.into_values().collect(),
            service: if services.is_empty() { None } else { Some(services.into_values().collect()) },
            authentication: Some(authentication.into_values().collect()),
            assertion_method: Some(assertion_method.into_values().collect()),
            key_agreement: None,
            capability_invocation: None,
            capability_delegation: None,
        };

        Ok(DIDResolution {
            did_document,
            did_document_metadata: DIDDocumentMetadata {
                created: created_at,
                updated: updated_at,
                deactivated: Some(deactivated),
            },
            did_resolution_metadata: DIDResolutionMetadata {
                content_type: "application/ld+json;profile=\"https://w3id.org/did-resolution\""
                    .to_string(),
            },
        })
    }
}

fn apply_did_owner(
    event: &DIDOwnerEvent,
    did_string: &str,
    verification_methods: &mut HashMap<String, VerificationMethod>,
    controller: &mut Option<String>,
) {
    let vm_id = format!("{}{}", did_string, DID_ROOT_KEY_ID);
    let vm = VerificationMethod::Multibase(VerificationMethodMultibase {
        id: vm_id.clone(),
        key_type: "Ed25519VerificationKey2020".to_string(),
        controller: event.did_owner.controller.clone(),
        public_key_multibase: event.did_owner.public_key_multibase.clone(),
    });
    *controller = Some(event.did_owner.controller.clone());
    verification_methods.insert(vm_id, vm);
}

#[cfg(test)]
mod tests {
    use super::DidDocumentBuilder;
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
    use hiero_did_core::{DIDError, HederaDid, KeysUtility, did::Network};
    use hiero_did_messages::{DIDOwnerMessage, HcsEnvelope, HcsMessage};
    use hiero_did_signer::InternalSigner;

    fn fixture() -> (HederaDid, InternalSigner) {
        let signer = InternalSigner::from_bytes(&[3u8; 32]).expect("signer");
        let did = HederaDid::new(
            Network::Testnet,
            KeysUtility::from_bytes(signer.verifying_key_bytes()).to_base58(),
            "0.0.321".to_string(),
        );
        (did, signer)
    }

    fn signed_create_payload(did: &HederaDid, signer: &InternalSigner) -> String {
        let msg = DIDOwnerMessage::new(did.clone(), signer.verifying_key_bytes(), None);
        let bytes = msg.message_bytes().expect("message bytes");
        let sig = signer.sign(&bytes);
        msg.to_payload(&sig).expect("payload")
    }

    fn signed_delete_payload(did: &HederaDid, signer: &InternalSigner) -> String {
        let message = HcsMessage {
            timestamp: "2026-01-01T00:00:01.000Z".to_string(),
            operation: "delete".to_string(),
            did: did.to_string(),
            event: String::new(),
        };
        let msg_bytes = serde_json::to_vec(&message).expect("serialize");
        let sig = signer.sign(&msg_bytes);
        let envelope = HcsEnvelope {
            message,
            signature: BASE64.encode(sig),
        };
        serde_json::to_string(&envelope).expect("serialize envelope")
    }

    #[tokio::test]
    async fn resolve_create_message() {
        let (did, signer) = fixture();
        let payload = signed_create_payload(&did, &signer);
        let resolution = DidDocumentBuilder::from(vec![payload])
            .resolve(&did)
            .await
            .expect("must resolve");

        assert_eq!(resolution.did_document.id, did.to_string());
        assert_eq!(resolution.did_document_metadata.deactivated, Some(false));
        assert!(!resolution.did_document.verification_method.is_empty());
    }

    #[tokio::test]
    async fn skip_invalid_signature_and_return_not_found() {
        let (did, signer) = fixture();
        let payload = signed_create_payload(&did, &signer);
        let mut envelope: HcsEnvelope = serde_json::from_str(&payload).expect("envelope");
        envelope.signature = BASE64.encode([0u8; 64]);
        let tampered = serde_json::to_string(&envelope).expect("serialize");

        let err = DidDocumentBuilder::from(vec![tampered])
            .resolve(&did)
            .await
            .expect_err("must fail");
        assert!(matches!(err, DIDError::NotFound(_)));
    }

    #[tokio::test]
    async fn delete_event_marks_document_deactivated() {
        let (did, signer) = fixture();
        let create = signed_create_payload(&did, &signer);
        let delete = signed_delete_payload(&did, &signer);
        let resolution = DidDocumentBuilder::from(vec![create, delete])
            .resolve(&did)
            .await
            .expect("must resolve");
        assert_eq!(resolution.did_document_metadata.deactivated, Some(true));
    }

    #[tokio::test]
    async fn unrelated_did_messages_are_filtered() {
        let (target_did, signer) = fixture();
        let other_did = HederaDid::new(
            Network::Testnet,
            target_did.base58_key.clone(),
            "0.0.999".to_string(),
        );
        let payload = signed_create_payload(&other_did, &signer);
        let err = DidDocumentBuilder::from(vec![payload])
            .resolve(&target_did)
            .await
            .expect_err("must fail");
        assert!(matches!(err, DIDError::NotFound(_)));
    }
}
