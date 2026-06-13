#[path = "create_csm.rs"]
pub mod create;
#[path = "deactivate_csm.rs"]
pub mod deactivate;
#[path = "update_csm.rs"]
pub mod update;

pub use create::prepare_create_did_csm;
pub use create::prepare_create_did_csm_with_options;
pub use create::submit_create_did_csm;
pub use deactivate::prepare_deactivate_did_csm;
pub use deactivate::prepare_deactivate_did_csm_with_options;
pub use deactivate::submit_deactivate_did_csm;
pub use update::prepare_update_did_csm;
pub use update::prepare_update_did_csm_with_options;
pub use update::submit_update_did_csm;

use hiero_did_core::DIDError;
use hiero_did_core::HederaDid;
use hiero_did_core::KeysUtility;
use hiero_did_hcs::HcsTopic;
use hiero_did_messages::DIDAddServiceMessage;
use hiero_did_messages::DIDAddVerificationMethodMessage;
use hiero_did_messages::DIDDeactivateMessage;
use hiero_did_messages::DIDOwnerMessage;
use hiero_did_messages::DIDRemoveServiceMessage;
use hiero_did_messages::DIDRemoveVerificationMethodMessage;
use hiero_did_signer::InternalVerifier;
use hiero_sdk::Client;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::str::FromStr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub const CSM_STATE_VERSION: u16 = 1;
pub const PAUSE_FOR_SIGNATURE_LABEL: &str = "pause-for-signature";
pub const PAUSE_BEFORE_PUBLISH_LABEL: &str = "pause-before-publish";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmSigningRequest {
    pub version: u16,
    pub request_id: String,
    pub did: String,
    pub topic_id: String,
    pub operation: String,
    pub lifecycle_label: String,
    pub message_bytes: Vec<u8>,
    pub state: CsmOperationState,
}

impl CsmSigningRequest {
    pub fn into_submit_request(self, signature: Vec<u8>) -> Result<CsmSubmitRequest, DIDError> {
        require_signature(&signature)?;
        self.state.validate_for_submit(&signature)?;
        Ok(CsmSubmitRequest {
            state: self.state,
            signature,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmBatchSigningRequest {
    pub version: u16,
    pub did: String,
    pub topic_id: String,
    pub operation: String,
    pub lifecycle_label: String,
    pub requests: Vec<CsmSigningRequest>,
}

impl CsmBatchSigningRequest {
    pub fn into_submit_request(
        self,
        signatures: Vec<Vec<u8>>,
    ) -> Result<CsmBatchSubmitRequest, DIDError> {
        if self.requests.len() != signatures.len() {
            return Err(DIDError::InvalidArgument(format!(
                "CSM signature count mismatch: expected {}, got {}",
                self.requests.len(),
                signatures.len()
            )));
        }

        let requests = self
            .requests
            .into_iter()
            .zip(signatures)
            .map(|(request, signature)| request.into_submit_request(signature))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(CsmBatchSubmitRequest { requests })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmOperationState {
    pub version: u16,
    pub request_id: String,
    pub did: String,
    pub topic_id: String,
    pub operation: String,
    pub lifecycle_label: String,
    pub expires_at_unix: Option<i64>,
    pub expected_public_key_bytes: Vec<u8>,
    pub message_bytes: Vec<u8>,
    pub message: CsmMessageState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CsmPrepareOptions {
    pub expires_at_unix: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CsmMessageState {
    DidOwner {
        did: String,
        public_key_bytes: Vec<u8>,
        timestamp: String,
        controller: Option<String>,
    },
    AddVerificationMethod {
        did: String,
        id: String,
        property: String,
        controller: String,
        public_key_multibase: String,
        timestamp: String,
    },
    RemoveVerificationMethod {
        did: String,
        id: String,
        timestamp: String,
    },
    AddService {
        did: String,
        id: String,
        service_type: String,
        service_endpoint: String,
        timestamp: String,
    },
    RemoveService {
        did: String,
        id: String,
        timestamp: String,
    },
    Deactivate {
        did: String,
        timestamp: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmSignature {
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmSubmitRequest {
    pub state: CsmOperationState,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmBatchSubmitRequest {
    pub requests: Vec<CsmSubmitRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmSubmitResult {
    pub did: String,
    pub topic_id: String,
    pub operation: String,
    pub sequence_number: u64,
    pub lifecycle_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmBatchSubmitResult {
    pub did: String,
    pub topic_id: String,
    pub operation: String,
    pub operations_applied: usize,
    pub results: Vec<CsmSubmitResult>,
}

impl CsmOperationState {
    pub fn signing_request(self) -> Result<CsmSigningRequest, DIDError> {
        self.validate_state()?;
        let message_bytes = self.rebuild_message_bytes()?;
        if message_bytes != self.message_bytes {
            return Err(DIDError::InvalidArgument(
                "CSM state message bytes do not match rebuilt message bytes".to_string(),
            ));
        }

        Ok(CsmSigningRequest {
            version: self.version,
            request_id: self.request_id.clone(),
            did: self.did.clone(),
            topic_id: self.topic_id.clone(),
            operation: self.operation.clone(),
            lifecycle_label: self.lifecycle_label.clone(),
            message_bytes,
            state: self,
        })
    }

    pub fn rebuild_message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        match &self.message {
            CsmMessageState::DidOwner {
                did,
                public_key_bytes,
                timestamp,
                controller,
            } => DIDOwnerMessage {
                did: parse_did(did)?,
                public_key_bytes: public_key_bytes.clone(),
                timestamp: timestamp.clone(),
                controller: controller.clone(),
                signature: None,
            }
            .message_bytes(),
            CsmMessageState::AddVerificationMethod {
                did,
                id,
                property,
                controller,
                public_key_multibase,
                timestamp,
            } => DIDAddVerificationMethodMessage {
                did: did.clone(),
                id: id.clone(),
                property: property.clone(),
                controller: controller.clone(),
                public_key_multibase: public_key_multibase.clone(),
                timestamp: timestamp.clone(),
            }
            .message_bytes(),
            CsmMessageState::RemoveVerificationMethod { did, id, timestamp } => {
                DIDRemoveVerificationMethodMessage {
                    did: did.clone(),
                    id: id.clone(),
                    timestamp: timestamp.clone(),
                }
                .message_bytes()
            }
            CsmMessageState::AddService {
                did,
                id,
                service_type,
                service_endpoint,
                timestamp,
            } => DIDAddServiceMessage {
                did: did.clone(),
                id: id.clone(),
                service_type: service_type.clone(),
                service_endpoint: service_endpoint.clone(),
                timestamp: timestamp.clone(),
            }
            .message_bytes(),
            CsmMessageState::RemoveService { did, id, timestamp } => DIDRemoveServiceMessage {
                did: did.clone(),
                id: id.clone(),
                timestamp: timestamp.clone(),
            }
            .message_bytes(),
            CsmMessageState::Deactivate { did, timestamp } => DIDDeactivateMessage {
                did: parse_did(did)?,
                timestamp: timestamp.clone(),
            }
            .message_bytes(),
        }
    }

    pub fn to_payload(&self, signature: &[u8]) -> Result<String, DIDError> {
        self.validate_for_submit(signature)?;

        match &self.message {
            CsmMessageState::DidOwner {
                did,
                public_key_bytes,
                timestamp,
                controller,
            } => DIDOwnerMessage {
                did: parse_did(did)?,
                public_key_bytes: public_key_bytes.clone(),
                timestamp: timestamp.clone(),
                controller: controller.clone(),
                signature: None,
            }
            .to_payload(signature),
            CsmMessageState::AddVerificationMethod {
                did,
                id,
                property,
                controller,
                public_key_multibase,
                timestamp,
            } => DIDAddVerificationMethodMessage {
                did: did.clone(),
                id: id.clone(),
                property: property.clone(),
                controller: controller.clone(),
                public_key_multibase: public_key_multibase.clone(),
                timestamp: timestamp.clone(),
            }
            .to_payload(signature),
            CsmMessageState::RemoveVerificationMethod { did, id, timestamp } => {
                DIDRemoveVerificationMethodMessage {
                    did: did.clone(),
                    id: id.clone(),
                    timestamp: timestamp.clone(),
                }
                .to_payload(signature)
            }
            CsmMessageState::AddService {
                did,
                id,
                service_type,
                service_endpoint,
                timestamp,
            } => DIDAddServiceMessage {
                did: did.clone(),
                id: id.clone(),
                service_type: service_type.clone(),
                service_endpoint: service_endpoint.clone(),
                timestamp: timestamp.clone(),
            }
            .to_payload(signature),
            CsmMessageState::RemoveService { did, id, timestamp } => DIDRemoveServiceMessage {
                did: did.clone(),
                id: id.clone(),
                timestamp: timestamp.clone(),
            }
            .to_payload(signature),
            CsmMessageState::Deactivate { did, timestamp } => DIDDeactivateMessage {
                did: parse_did(did)?,
                timestamp: timestamp.clone(),
            }
            .to_payload(signature),
        }
    }

    fn require_matching_message_bytes(&self) -> Result<(), DIDError> {
        let rebuilt = self.rebuild_message_bytes()?;
        if rebuilt != self.message_bytes {
            return Err(DIDError::InvalidArgument(
                "CSM state message bytes do not match rebuilt message bytes".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_for_submit(&self, signature: &[u8]) -> Result<(), DIDError> {
        self.validate_state()?;
        require_signature(signature)?;
        self.require_matching_message_bytes()?;
        self.require_not_expired(current_unix_timestamp()?)?;
        self.verify_signature(signature)
    }

    pub fn validate_state(&self) -> Result<(), DIDError> {
        if self.version != CSM_STATE_VERSION {
            return Err(DIDError::InvalidArgument(format!(
                "Unsupported CSM state version: {}",
                self.version
            )));
        }

        let expected_request_id = build_request_id(
            &self.did,
            &self.topic_id,
            &self.operation,
            &self.message_bytes,
        );
        if self.request_id != expected_request_id {
            return Err(DIDError::InvalidArgument(
                "CSM request_id does not match operation state".to_string(),
            ));
        }

        if self.expected_public_key_bytes.len() != 32 {
            return Err(DIDError::InvalidArgument(format!(
                "CSM expected public key must be 32 bytes, got {}",
                self.expected_public_key_bytes.len()
            )));
        }

        Ok(())
    }

    pub fn require_not_expired(&self, now_unix: i64) -> Result<(), DIDError> {
        if let Some(expires_at) = self.expires_at_unix {
            if now_unix > expires_at {
                return Err(DIDError::InvalidArgument(format!(
                    "CSM request expired at {expires_at}"
                )));
            }
        }

        Ok(())
    }

    pub fn verify_signature(&self, signature: &[u8]) -> Result<(), DIDError> {
        let verifier = InternalVerifier::from_bytes(&self.expected_public_key_bytes)?;
        if !verifier.verify(&self.message_bytes, signature)? {
            return Err(DIDError::InvalidSignature(
                "CSM signature does not verify against expected public key".to_string(),
            ));
        }
        Ok(())
    }
}

pub fn require_signature(signature: &[u8]) -> Result<(), DIDError> {
    if signature.is_empty() {
        return Err(DIDError::InvalidArgument(
            "CSM signature cannot be empty".to_string(),
        ));
    }

    if signature.len() != 64 {
        return Err(DIDError::InvalidArgument(format!(
            "CSM signature must be 64 bytes, got {}",
            signature.len()
        )));
    }

    Ok(())
}

pub async fn submit_csm_request(
    client: &Client,
    request: CsmSubmitRequest,
) -> Result<CsmSubmitResult, DIDError> {
    request.state.validate_for_submit(&request.signature)?;
    let topic_id = request
        .state
        .topic_id
        .parse()
        .map_err(|e| DIDError::InvalidDid(format!("Cannot parse CSM topic ID: {e}")))?;
    let payload = request.state.to_payload(&request.signature)?;
    let submit = HcsTopic::submit(client, topic_id, payload).await?;

    Ok(CsmSubmitResult {
        did: request.state.did,
        topic_id: submit.topic_id,
        operation: request.state.operation,
        sequence_number: submit.sequence_number,
        lifecycle_label: PAUSE_BEFORE_PUBLISH_LABEL.to_string(),
    })
}

pub async fn submit_csm_batch(
    client: &Client,
    request: CsmBatchSubmitRequest,
) -> Result<CsmBatchSubmitResult, DIDError> {
    let mut results = Vec::with_capacity(request.requests.len());

    for item in request.requests {
        results.push(submit_csm_request(client, item).await?);
    }

    let first = results.first().ok_or_else(|| {
        DIDError::InvalidArgument("CSM batch submit request cannot be empty".to_string())
    })?;

    Ok(CsmBatchSubmitResult {
        did: first.did.clone(),
        topic_id: first.topic_id.clone(),
        operation: first.operation.clone(),
        operations_applied: results.len(),
        results,
    })
}

pub(crate) fn parse_did(did: &str) -> Result<HederaDid, DIDError> {
    HederaDid::from_str(did)
}

pub(crate) fn build_state(
    did: String,
    topic_id: String,
    operation: String,
    message: CsmMessageState,
    expected_public_key_bytes: Vec<u8>,
    options: CsmPrepareOptions,
) -> Result<CsmOperationState, DIDError> {
    let mut state = CsmOperationState {
        version: CSM_STATE_VERSION,
        request_id: String::new(),
        did,
        topic_id,
        operation,
        lifecycle_label: PAUSE_FOR_SIGNATURE_LABEL.to_string(),
        expires_at_unix: options.expires_at_unix,
        expected_public_key_bytes,
        message_bytes: Vec::new(),
        message,
    };
    state.message_bytes = state.rebuild_message_bytes()?;
    state.request_id = build_request_id(
        &state.did,
        &state.topic_id,
        &state.operation,
        &state.message_bytes,
    );
    Ok(state)
}

pub(crate) fn public_key_from_did(did: &HederaDid) -> Result<Vec<u8>, DIDError> {
    let key = KeysUtility::from_base58(&did.base58_key)?;
    let bytes = key.to_bytes().to_vec();
    if bytes.len() != 32 {
        return Err(DIDError::InvalidArgument(format!(
            "DID public key must be 32 bytes, got {}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn build_request_id(did: &str, topic_id: &str, operation: &str, message_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(did.as_bytes());
    hasher.update([0]);
    hasher.update(topic_id.as_bytes());
    hasher.update([0]);
    hasher.update(operation.as_bytes());
    hasher.update([0]);
    hasher.update(message_bytes);
    hex::encode(hasher.finalize())
}

fn current_unix_timestamp() -> Result<i64, DIDError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .map_err(|e| DIDError::InternalError(format!("System time before Unix epoch: {e}")))
}

#[cfg(test)]
mod tests {
    use super::deactivate::prepare_deactivate_did_csm;
    use super::deactivate::prepare_deactivate_did_csm_with_options;
    use super::require_signature;
    use super::update::prepare_update_did_csm;
    use super::*;
    use crate::update::AddService;
    use crate::update::DIDUpdateOperation;
    use hiero_did_core::did::Network;
    use hiero_did_signer::InternalSigner;

    fn test_signer() -> InternalSigner {
        InternalSigner::from_bytes(&[9u8; 32]).expect("signer")
    }

    fn test_did(signer: &InternalSigner) -> HederaDid {
        let base58_key = KeysUtility::from_bytes(signer.verifying_key_bytes()).to_base58();
        HederaDid::new(Network::Testnet, base58_key, "0.0.123".to_string())
    }

    #[test]
    fn signature_validation_requires_ed25519_signature_length() {
        assert!(require_signature(&[]).is_err());
        assert!(require_signature(&[1u8; 63]).is_err());
        assert!(require_signature(&[1u8; 64]).is_ok());
    }

    #[tokio::test]
    async fn deactivate_prepare_preserves_exact_message_bytes() {
        let signer = test_signer();
        let request = prepare_deactivate_did_csm(test_did(&signer)).await.expect("prepare");

        assert_eq!(request.version, CSM_STATE_VERSION);
        assert_eq!(request.request_id, request.state.request_id);
        assert_eq!(request.operation, "delete");
        assert_eq!(request.lifecycle_label, PAUSE_FOR_SIGNATURE_LABEL);
        assert_eq!(
            request.message_bytes,
            request.state.rebuild_message_bytes().expect("bytes")
        );
    }

    #[tokio::test]
    async fn update_prepare_builds_one_request_per_operation() {
        let signer = test_signer();
        let did = test_did(&signer);
        let request = prepare_update_did_csm(
            did.clone(),
            vec![DIDUpdateOperation::AddService(AddService {
                id: format!("{did}#svc"),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com".to_string(),
            })],
        )
        .await
        .expect("prepare");

        assert_eq!(request.operation, "update");
        assert_eq!(request.requests.len(), 1);
        assert_eq!(
            request.requests[0].message_bytes,
            request.requests[0]
                .state
                .rebuild_message_bytes()
                .expect("bytes")
        );
    }

    #[tokio::test]
    async fn signing_request_converts_to_submit_request() {
        let signer = test_signer();
        let request = prepare_deactivate_did_csm(test_did(&signer)).await.expect("prepare");
        let signature = signer.sign(&request.message_bytes);
        let submit = request
            .clone()
            .into_submit_request(signature.clone())
            .expect("submit request");

        assert_eq!(submit.state.did, request.did);
        assert_eq!(submit.signature, signature);
    }

    #[tokio::test]
    async fn batch_signing_request_rejects_signature_count_mismatch() {
        let signer = test_signer();
        let did = test_did(&signer);
        let request = prepare_update_did_csm(
            did.clone(),
            vec![DIDUpdateOperation::AddService(AddService {
                id: format!("{did}#svc"),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com".to_string(),
            })],
        )
        .await
        .expect("prepare");

        let err = request.into_submit_request(vec![]).expect_err("must fail");
        assert!(matches!(err, DIDError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn submit_request_rejects_invalid_signature() {
        let signer = test_signer();
        let request = prepare_deactivate_did_csm(test_did(&signer)).await.expect("prepare");
        let err = request
            .into_submit_request(vec![1u8; 64])
            .expect_err("must fail");

        assert!(matches!(err, DIDError::InvalidSignature(_)));
    }

    #[tokio::test]
    async fn expired_state_is_rejected_before_submit() {
        let signer = test_signer();
        let request = prepare_deactivate_did_csm_with_options(
            test_did(&signer),
            CsmPrepareOptions {
                expires_at_unix: Some(1),
            },
        )
        .await
        .expect("prepare");
        let signature = signer.sign(&request.message_bytes);
        let err = request
            .into_submit_request(signature)
            .expect_err("must fail");

        assert!(matches!(err, DIDError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn serialized_signing_request_preserves_exact_bytes() {
        let signer = test_signer();
        let request = prepare_deactivate_did_csm(test_did(&signer)).await.expect("prepare");
        let json = serde_json::to_string(&request).expect("serialize");
        let decoded: CsmSigningRequest = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(decoded.message_bytes, request.message_bytes);
        assert_eq!(decoded.request_id, request.request_id);

        let signature = signer.sign(&decoded.message_bytes);
        decoded
            .into_submit_request(signature)
            .expect("submit request");
    }
}
