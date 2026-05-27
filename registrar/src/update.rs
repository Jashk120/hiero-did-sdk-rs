use hiero_did_core::signer::Signer;
use hiero_did_core::{DIDError, HederaDid};
use hiero_did_hcs::HcsTopic;
use hiero_did_messages::{
    DIDAddServiceMessage, DIDAddVerificationMethodMessage, DIDRemoveServiceMessage,
    DIDRemoveVerificationMethodMessage,
};
use hiero_did_signer::InternalSigner;
use hiero_sdk::Client;

#[derive(Debug, Clone)]
pub enum VerificationMethodProperty {
    VerificationMethod,
    Authentication,
    AssertionMethod,
    KeyAgreement,
    CapabilityInvocation,
    CapabilityDelegation,
}

impl VerificationMethodProperty {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VerificationMethod => "verificationMethod",
            Self::Authentication => "authentication",
            Self::AssertionMethod => "assertionMethod",
            Self::KeyAgreement => "keyAgreement",
            Self::CapabilityInvocation => "capabilityInvocation",
            Self::CapabilityDelegation => "capabilityDelegation",
        }
    }
}

pub struct AddVerificationMethod {
    /// Fragment id, e.g. `#key-1`
    pub id: String,
    pub property: VerificationMethodProperty,
    /// Defaults to the DID itself if None
    pub controller: Option<String>,
    /// Required when property == VerificationMethod; optional for aliases
    pub public_key_multibase: Option<String>,
}

pub struct RemoveVerificationMethod {
    pub id: String,
}

pub struct AddService {
    /// Fragment id, e.g. `#vcs`
    pub id: String,
    pub service_type: String,
    pub service_endpoint: String,
}

pub struct RemoveService {
    pub id: String,
}

pub enum DIDUpdateOperation {
    AddVerificationMethod(AddVerificationMethod),
    RemoveVerificationMethod(RemoveVerificationMethod),
    AddService(AddService),
    RemoveService(RemoveService),
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

pub struct UpdateDIDResult {
    pub did: String,
    pub operations_applied: usize,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Updates an existing did:hedera DID document.
///
/// Mirrors JS `updateDID` (internal signer flow). Each operation is a
/// separate HCS message on the same topic, submitted in order.
pub async fn update_did(
    client: &Client,
    did: HederaDid,
    private_key_bytes: &[u8],
    updates: Vec<DIDUpdateOperation>,
) -> Result<UpdateDIDResult, DIDError> {
    let did_str = did.to_string();

    // mirrors JS: if updates.length === 0 return early
    if updates.is_empty() {
        return Ok(UpdateDIDResult {
            did: did_str,
            operations_applied: 0,
        });
    }

    let topic_id = did
        .topic_id
        .parse()
        .map_err(|e| DIDError::InvalidDid(format!("Cannot parse topic ID from DID: {}", e)))?;

    let signer = InternalSigner::from_raw_bytes(private_key_bytes)?;
    let mut applied = 0usize;

    for op in updates {
        apply_operation(client, &did_str, topic_id, &signer, op).await?;
        applied += 1;
    }

    Ok(UpdateDIDResult {
        did: did_str,
        operations_applied: applied,
    })
}

// ---------------------------------------------------------------------------
// Dispatcher  (mirrors JS OPERATIONS_MAP)
// ---------------------------------------------------------------------------

async fn apply_operation(
    client: &Client,
    did_str: &str,
    topic_id: hiero_sdk::TopicId,
    signer: &dyn Signer,
    op: DIDUpdateOperation,
) -> Result<(), DIDError> {
    match op {
        DIDUpdateOperation::AddVerificationMethod(opts) => {
            apply_add_verification_method(client, did_str, topic_id, signer, opts).await
        }
        DIDUpdateOperation::RemoveVerificationMethod(opts) => {
            apply_remove_verification_method(client, did_str, topic_id, signer, opts).await
        }
        DIDUpdateOperation::AddService(opts) => {
            apply_add_service(client, did_str, topic_id, signer, opts).await
        }
        DIDUpdateOperation::RemoveService(opts) => {
            apply_remove_service(client, did_str, topic_id, signer, opts).await
        }
    }
}

// ---------------------------------------------------------------------------
// Sub-operation handlers
// ---------------------------------------------------------------------------

async fn apply_add_verification_method(
    client: &Client,
    did_str: &str,
    topic_id: hiero_sdk::TopicId,
    signer: &dyn Signer,
    opts: AddVerificationMethod,
) -> Result<(), DIDError> {
    if matches!(
        opts.property,
        VerificationMethodProperty::VerificationMethod
    ) && opts.public_key_multibase.is_none()
    {
        return Err(DIDError::InvalidArgument(
            "public_key_multibase is required for verificationMethod property".into(),
        ));
    }
    let controller = opts.controller.unwrap_or_else(|| did_str.to_string());
    let message = DIDAddVerificationMethodMessage::new(
        did_str.to_string(),
        opts.id,
        opts.property.as_str().to_string(),
        controller,
        opts.public_key_multibase.unwrap_or_default(),
    );
    sign_and_submit(client, topic_id, signer, &message).await
}

async fn apply_remove_verification_method(
    client: &Client,
    did_str: &str,
    topic_id: hiero_sdk::TopicId,
    signer: &dyn Signer,
    opts: RemoveVerificationMethod,
) -> Result<(), DIDError> {
    let message = DIDRemoveVerificationMethodMessage::new(did_str.to_string(), opts.id);
    sign_and_submit(client, topic_id, signer, &message).await
}

async fn apply_add_service(
    client: &Client,
    did_str: &str,
    topic_id: hiero_sdk::TopicId,
    signer: &dyn Signer,
    opts: AddService,
) -> Result<(), DIDError> {
    let message = DIDAddServiceMessage::new(
        did_str.to_string(),
        opts.id,
        opts.service_type,
        opts.service_endpoint,
    );
    sign_and_submit(client, topic_id, signer, &message).await
}

async fn apply_remove_service(
    client: &Client,
    did_str: &str,
    topic_id: hiero_sdk::TopicId,
    signer: &dyn Signer,
    opts: RemoveService,
) -> Result<(), DIDError> {
    let message = DIDRemoveServiceMessage::new(did_str.to_string(), opts.id);
    sign_and_submit(client, topic_id, signer, &message).await
}

// ---------------------------------------------------------------------------
// Shared sign + submit helper
// ---------------------------------------------------------------------------

/// sign_and_submit collapses the JS LifecycleRunner boilerplate into:
/// message_bytes → sign → to_payload → HcsTopic::submit
/// This matches exactly how create.rs works with DIDOwnerMessage.
async fn sign_and_submit<M: HcsSignable>(
    client: &Client,
    topic_id: hiero_sdk::TopicId,
    signer: &dyn Signer,
    message: &M,
) -> Result<(), DIDError> {
    let msg_bytes = message.message_bytes()?;
    let signature = signer.sign_bytes(&msg_bytes)?;
    let payload = message.to_payload(&signature)?;
    HcsTopic::submit(client, topic_id, payload).await?;
    Ok(())
}

/// Shared surface every DID*Message type satisfies.
/// Matches the methods already on DIDOwnerMessage in create.rs.
pub trait HcsSignable {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError>;
    fn to_payload(&self, signature: &[u8]) -> Result<String, DIDError>;
}

impl HcsSignable for DIDAddVerificationMethodMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        self.message_bytes()
    }
    fn to_payload(&self, sig: &[u8]) -> Result<String, DIDError> {
        self.to_payload(sig)
    }
}
impl HcsSignable for DIDRemoveVerificationMethodMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        self.message_bytes()
    }
    fn to_payload(&self, sig: &[u8]) -> Result<String, DIDError> {
        self.to_payload(sig)
    }
}
impl HcsSignable for DIDAddServiceMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        self.message_bytes()
    }
    fn to_payload(&self, sig: &[u8]) -> Result<String, DIDError> {
        self.to_payload(sig)
    }
}
impl HcsSignable for DIDRemoveServiceMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        self.message_bytes()
    }
    fn to_payload(&self, sig: &[u8]) -> Result<String, DIDError> {
        self.to_payload(sig)
    }
}
pub async fn update_did_with_signer(
    client: &Client,
    did: HederaDid,
    signer: &dyn Signer,
    updates: Vec<DIDUpdateOperation>,
) -> Result<UpdateDIDResult, DIDError> {
    let did_str = did.to_string();

    if updates.is_empty() {
        return Ok(UpdateDIDResult {
            did: did_str,
            operations_applied: 0,
        });
    }

    let topic_id = did
        .topic_id
        .parse()
        .map_err(|e| DIDError::InvalidDid(format!("Cannot parse topic ID from DID: {}", e)))?;

    let mut applied = 0usize;
    for op in updates {
        apply_operation(client, &did_str, topic_id, signer, op).await?;
        applied += 1;
    }

    Ok(UpdateDIDResult {
        did: did_str,
        operations_applied: applied,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AddVerificationMethod, DIDUpdateOperation, VerificationMethodProperty, update_did,
        update_did_with_signer,
    };
    use hiero_did_core::Signer;
    use hiero_did_core::{DIDError, HederaDid, did::Network};
    use hiero_sdk::Client;

    struct TestSigner;

    impl Signer for TestSigner {
        fn public_key_bytes(&self) -> Vec<u8> {
            vec![1u8; 32]
        }

        fn sign_bytes(&self, _message: &[u8]) -> Result<Vec<u8>, DIDError> {
            Ok(vec![2u8; 64])
        }
    }

    #[test]
    fn verification_method_property_mapping() {
        assert_eq!(
            VerificationMethodProperty::VerificationMethod.as_str(),
            "verificationMethod"
        );
        assert_eq!(
            VerificationMethodProperty::Authentication.as_str(),
            "authentication"
        );
        assert_eq!(
            VerificationMethodProperty::AssertionMethod.as_str(),
            "assertionMethod"
        );
        assert_eq!(
            VerificationMethodProperty::KeyAgreement.as_str(),
            "keyAgreement"
        );
        assert_eq!(
            VerificationMethodProperty::CapabilityInvocation.as_str(),
            "capabilityInvocation"
        );
        assert_eq!(
            VerificationMethodProperty::CapabilityDelegation.as_str(),
            "capabilityDelegation"
        );
    }

    #[tokio::test]
    async fn update_with_empty_ops_returns_early() {
        let client = Client::for_testnet();
        let did = HederaDid::new(Network::Testnet, "abc".to_string(), "0.0.123".to_string());
        let out = update_did(&client, did.clone(), &[1u8; 32], vec![])
            .await
            .expect("success");
        assert_eq!(out.did, did.to_string());
        assert_eq!(out.operations_applied, 0);
    }

    #[tokio::test]
    async fn update_with_signer_empty_ops_returns_early() {
        let client = Client::for_testnet();
        let did = HederaDid::new(Network::Testnet, "abc".to_string(), "0.0.123".to_string());
        let signer = TestSigner;

        let out = update_did_with_signer(&client, did.clone(), &signer, vec![])
            .await
            .expect("success");

        assert_eq!(out.did, did.to_string());
        assert_eq!(out.operations_applied, 0);
    }

    #[tokio::test]
    async fn update_rejects_missing_verification_method_key() {
        let client = Client::for_testnet();
        let did = HederaDid::new(Network::Testnet, "abc".to_string(), "0.0.123".to_string());
        let op = DIDUpdateOperation::AddVerificationMethod(AddVerificationMethod {
            id: format!("{did}#key-1"),
            property: VerificationMethodProperty::VerificationMethod,
            controller: None,
            public_key_multibase: None,
        });
        let err = match update_did(&client, did, &[1u8; 32], vec![op]).await {
            Ok(_) => panic!("must fail"),
            Err(e) => e,
        };
        assert!(matches!(err, DIDError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn update_with_signer_rejects_invalid_topic_before_signing() {
        let client = Client::for_testnet();
        let did = HederaDid::new(Network::Testnet, "abc".to_string(), "bad-topic".to_string());
        let signer = TestSigner;
        let op = DIDUpdateOperation::RemoveService(super::RemoveService {
            id: format!("{did}#svc-1"),
        });

        let err = match update_did_with_signer(&client, did, &signer, vec![op]).await {
            Ok(_) => panic!("must fail"),
            Err(e) => e,
        };

        assert!(matches!(err, DIDError::InvalidDid(_)));
    }

    #[tokio::test]
    async fn update_with_signer_rejects_missing_verification_method_key() {
        let client = Client::for_testnet();
        let did = HederaDid::new(Network::Testnet, "abc".to_string(), "0.0.123".to_string());
        let signer = TestSigner;
        let op = DIDUpdateOperation::AddVerificationMethod(AddVerificationMethod {
            id: format!("{did}#key-1"),
            property: VerificationMethodProperty::VerificationMethod,
            controller: None,
            public_key_multibase: None,
        });

        let err = match update_did_with_signer(&client, did, &signer, vec![op]).await {
            Ok(_) => panic!("must fail"),
            Err(e) => e,
        };

        assert!(matches!(err, DIDError::InvalidArgument(_)));
    }
}
