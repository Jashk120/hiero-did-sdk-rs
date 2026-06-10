use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hiero_did_core::{
    DIDError, DIDDocument, DIDDocumentMetadata, DIDResolution, DIDResolutionMetadata,
    KeyCapabilityMethod, KeysUtility, Service, VerificationMethod, VerificationMethodMultibase,
    did::{DID_ROOT_KEY_ID, HederaDid},
};
use hiero_did_messages::envelope::HcsEnvelope;
use hiero_did_messages::events::{
    DIDEvent,
    DIDOwnerEvent,
    DIDAddVerificationMethodEvent,
    DIDRemoveVerificationMethodEvent,
    DIDAddServiceEvent,
    DIDRemoveServiceEvent,
};
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
        let mut services: HashMap<String, Service> = HashMap::new();
        let mut authentication: HashMap<String, KeyCapabilityMethod> = HashMap::new();
        let mut assertion_method: HashMap<String, KeyCapabilityMethod> = HashMap::new();
        let mut key_agreement: HashMap<String, KeyCapabilityMethod> = HashMap::new();
        let mut capability_invocation: HashMap<String, KeyCapabilityMethod> = HashMap::new();
        let mut capability_delegation: HashMap<String, KeyCapabilityMethod> = HashMap::new();
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

            // handle delete — verify sig then tombstone
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
            let event_b64 = match &message.event {
                Some(e) => e,
                None => continue,  // no event to process (shouldn't happen for non-delete)
            };
            let event_json = match BASE64.decode(event_b64) {
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

            // verify signature — skip message if invalid
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

            // apply event to document state
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

                DIDEvent::AddVerificationMethod(vm_event) => {
                    apply_add_verification_method(
                        &vm_event,
                        &mut verification_methods,
                        &mut authentication,
                        &mut assertion_method,
                        &mut key_agreement,
                        &mut capability_invocation,
                        &mut capability_delegation,
                    );
                }

                DIDEvent::RemoveVerificationMethod(vm_event) => {
                    apply_remove_verification_method(
                        &vm_event,
                        &mut verification_methods,
                        &mut authentication,
                        &mut assertion_method,
                        &mut key_agreement,
                        &mut capability_invocation,
                        &mut capability_delegation,
                    );
                }

                DIDEvent::AddService(svc_event) => {
                    apply_add_service(&svc_event, &mut services);
                }

                DIDEvent::RemoveService(svc_event) => {
                    apply_remove_service(&svc_event, &mut services);
                }

                // Deactivate is handled above via operation == "delete"
                DIDEvent::Deactivate(_) => {}
            }

            updated_at = Some(message.timestamp.clone());
        }

        if !exists {
            return Err(DIDError::NotFound(format!("DID document not found: {}", did_string)));
        }

        // inject root key into authentication + assertionMethod if not already present
        let root_key_id = did.root_key_id();
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
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: did_string.clone(),
            controller: controller.unwrap_or_else(|| did_string.clone()),
            verification_method: verification_methods.into_values().collect(),
            service: if services.is_empty() { None } else { Some(services.into_values().collect()) },
            authentication: Some(authentication.into_values().collect()),
            assertion_method: Some(assertion_method.into_values().collect()),
            key_agreement: if key_agreement.is_empty() { None } else { Some(key_agreement.into_values().collect()) },
            capability_invocation: if capability_invocation.is_empty() { None } else { Some(capability_invocation.into_values().collect()) },
            capability_delegation: if capability_delegation.is_empty() { None } else { Some(capability_delegation.into_values().collect()) },
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

// ---------------------------------------------------------------------------
// Event applicators
// ---------------------------------------------------------------------------

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

fn apply_add_verification_method(
    event: &DIDAddVerificationMethodEvent,
    verification_methods: &mut HashMap<String, VerificationMethod>,
    authentication: &mut HashMap<String, KeyCapabilityMethod>,
    assertion_method: &mut HashMap<String, KeyCapabilityMethod>,
    key_agreement: &mut HashMap<String, KeyCapabilityMethod>,
    capability_invocation: &mut HashMap<String, KeyCapabilityMethod>,
    capability_delegation: &mut HashMap<String, KeyCapabilityMethod>,
) {
    let data = &event.verification_method;

    // upsert into verificationMethod if it has a public key
    if !data.public_key_multibase.is_empty() {
        let vm = VerificationMethod::Multibase(VerificationMethodMultibase {
            id: data.id.clone(),
            key_type: data.key_type.clone(),
            controller: data.controller.clone(),
            public_key_multibase: data.public_key_multibase.clone(),
        });
        verification_methods.insert(data.id.clone(), vm);
    }

    // insert reference into the correct relationship bucket
    let reference = KeyCapabilityMethod::Reference(data.id.clone());
    match data.relationship_type.as_str() {
        "authentication"        => { authentication.insert(data.id.clone(), reference); }
        "assertionMethod"       => { assertion_method.insert(data.id.clone(), reference); }
        "keyAgreement"          => { key_agreement.insert(data.id.clone(), reference); }
        "capabilityInvocation"  => { capability_invocation.insert(data.id.clone(), reference); }
        "capabilityDelegation"  => { capability_delegation.insert(data.id.clone(), reference); }
        // "verificationMethod" — key is already in verification_methods above, no relationship bucket
        _ => {}
    }
}

fn apply_remove_verification_method(
    event: &DIDRemoveVerificationMethodEvent,
    verification_methods: &mut HashMap<String, VerificationMethod>,
    authentication: &mut HashMap<String, KeyCapabilityMethod>,
    assertion_method: &mut HashMap<String, KeyCapabilityMethod>,
    key_agreement: &mut HashMap<String, KeyCapabilityMethod>,
    capability_invocation: &mut HashMap<String, KeyCapabilityMethod>,
    capability_delegation: &mut HashMap<String, KeyCapabilityMethod>,
) {
    let id = &event.verification_method.id;

    // remove from verificationMethod and all relationship buckets
    verification_methods.remove(id);
    authentication.remove(id);
    assertion_method.remove(id);
    key_agreement.remove(id);
    capability_invocation.remove(id);
    capability_delegation.remove(id);
}

fn apply_add_service(
    event: &DIDAddServiceEvent,
    services: &mut HashMap<String, Service>,
) {
    let data = &event.service;
    let svc = Service {
        id: data.id.clone(),
        service_type: data.service_type.clone(),
        service_endpoint: data.service_endpoint.clone(),
    };
    services.insert(data.id.clone(), svc);
}

fn apply_remove_service(
    event: &DIDRemoveServiceEvent,
    services: &mut HashMap<String, Service>,
) {
    services.remove(&event.service.id);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::DidDocumentBuilder;
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
    use hiero_did_core::{DIDError, HederaDid, KeysUtility, did::Network};
    use hiero_did_messages::{
        DIDOwnerMessage, HcsEnvelope,
        DIDAddVerificationMethodMessage, DIDAddServiceMessage,
        DIDRemoveVerificationMethodMessage, DIDRemoveServiceMessage,
        DIDDeactivateMessage,
    };
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

    fn signed_create(did: &HederaDid, signer: &InternalSigner) -> String {
        let msg = DIDOwnerMessage::new(did.clone(), signer.verifying_key_bytes(), None);
        let sig = signer.sign(&msg.message_bytes().unwrap());
        msg.to_payload(&sig).unwrap()
    }

    fn signed_delete(did: &HederaDid, signer: &InternalSigner) -> String {
        let msg = DIDDeactivateMessage::new(did.clone());
        let sig = signer.sign(&msg.message_bytes().unwrap());
        msg.to_payload(&sig).unwrap()
    }

    fn signed_add_vm(did: &HederaDid, signer: &InternalSigner, id: &str, property: &str) -> String {
        let msg = DIDAddVerificationMethodMessage::new(
            did.to_string(),
            id.to_string(),
            property.to_string(),
            did.to_string(),
            "zFakeMultibaseKey".to_string(),
        );
        let sig = signer.sign(&msg.message_bytes().unwrap());
        msg.to_payload(&sig).unwrap()
    }

    fn signed_remove_vm(did: &HederaDid, signer: &InternalSigner, id: &str) -> String {
        let msg = DIDRemoveVerificationMethodMessage::new(did.to_string(), id.to_string());
        let sig = signer.sign(&msg.message_bytes().unwrap());
        msg.to_payload(&sig).unwrap()
    }

    fn signed_add_service(did: &HederaDid, signer: &InternalSigner, id: &str) -> String {
        let msg = DIDAddServiceMessage::new(
            did.to_string(),
            id.to_string(),
            "VerifiableCredentialService".to_string(),
            "https://example.com/vc".to_string(),
        );
        let sig = signer.sign(&msg.message_bytes().unwrap());
        msg.to_payload(&sig).unwrap()
    }

    fn signed_remove_service(did: &HederaDid, signer: &InternalSigner, id: &str) -> String {
        let msg = DIDRemoveServiceMessage::new(did.to_string(), id.to_string());
        let sig = signer.sign(&msg.message_bytes().unwrap());
        msg.to_payload(&sig).unwrap()
    }

    #[tokio::test]
    async fn resolve_create_message() {
        let (did, signer) = fixture();
        let resolution = DidDocumentBuilder::from(vec![signed_create(&did, &signer)])
            .resolve(&did).await.unwrap();
        assert_eq!(resolution.did_document.id, did.to_string());
        assert_eq!(resolution.did_document_metadata.deactivated, Some(false));
        assert!(!resolution.did_document.verification_method.is_empty());
    }

    #[tokio::test]
    async fn deactivate_marks_document_deactivated() {
        let (did, signer) = fixture();
        let resolution = DidDocumentBuilder::from(vec![
            signed_create(&did, &signer),
            signed_delete(&did, &signer),
        ]).resolve(&did).await.unwrap();
        assert_eq!(resolution.did_document_metadata.deactivated, Some(true));
    }

    #[tokio::test]
    async fn add_and_remove_verification_method() {
        let (did, signer) = fixture();
        let vm_id = format!("{}#key-2", did);
        let resolution = DidDocumentBuilder::from(vec![
            signed_create(&did, &signer),
            signed_add_vm(&did, &signer, &vm_id, "authentication"),
        ]).resolve(&did).await.unwrap();

        let has_vm = resolution.did_document.verification_method
            .iter().any(|vm| vm.id() == vm_id);
        assert!(has_vm, "verification method should be present");

        let resolution2 = DidDocumentBuilder::from(vec![
            signed_create(&did, &signer),
            signed_add_vm(&did, &signer, &vm_id, "authentication"),
            signed_remove_vm(&did, &signer, &vm_id),
        ]).resolve(&did).await.unwrap();

        let has_vm2 = resolution2.did_document.verification_method
            .iter().any(|vm| vm.id() == vm_id);
        assert!(!has_vm2, "verification method should be removed");
    }

    #[tokio::test]
    async fn add_and_remove_service() {
        let (did, signer) = fixture();
        let svc_id = format!("{}#vcs", did);

        let resolution = DidDocumentBuilder::from(vec![
            signed_create(&did, &signer),
            signed_add_service(&did, &signer, &svc_id),
        ]).resolve(&did).await.unwrap();
        assert!(resolution.did_document.service.is_some());

        let resolution2 = DidDocumentBuilder::from(vec![
            signed_create(&did, &signer),
            signed_add_service(&did, &signer, &svc_id),
            signed_remove_service(&did, &signer, &svc_id),
        ]).resolve(&did).await.unwrap();
        assert!(resolution2.did_document.service.is_none());
    }

    #[tokio::test]
    async fn skip_invalid_signature() {
        let (did, signer) = fixture();
        let payload = signed_create(&did, &signer);
        let mut envelope: HcsEnvelope = serde_json::from_str(&payload).unwrap();
        envelope.signature = BASE64.encode([0u8; 64]);
        let tampered = serde_json::to_string(&envelope).unwrap();
        let err = DidDocumentBuilder::from(vec![tampered])
            .resolve(&did).await.expect_err("must fail");
        assert!(matches!(err, DIDError::NotFound(_)));
    }

    #[tokio::test]
    async fn unrelated_did_messages_are_filtered() {
        let (target_did, signer) = fixture();
        let other_did = HederaDid::new(
            Network::Testnet,
            target_did.base58_key.clone(),
            "0.0.999".to_string(),
        );
        let err = DidDocumentBuilder::from(vec![signed_create(&other_did, &signer)])
            .resolve(&target_did).await.expect_err("must fail");
        assert!(matches!(err, DIDError::NotFound(_)));
    }
}