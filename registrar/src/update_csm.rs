use crate::csm::CsmBatchSigningRequest;
use crate::csm::CsmBatchSubmitRequest;
use crate::csm::CsmBatchSubmitResult;
use crate::csm::CsmMessageState;
use crate::csm::CsmPrepareOptions;
use crate::csm::CsmSigningRequest;
use crate::csm::PAUSE_FOR_SIGNATURE_LABEL;
use crate::csm::build_state;
use crate::csm::public_key_from_did;
use crate::csm::submit_csm_batch;
use crate::update::DIDUpdateOperation;
use crate::update::VerificationMethodProperty;
use hiero_did_core::DIDError;
use hiero_did_core::HederaDid;
use hiero_did_messages::DIDAddServiceMessage;
use hiero_did_messages::DIDAddVerificationMethodMessage;
use hiero_did_messages::DIDRemoveServiceMessage;
use hiero_did_messages::DIDRemoveVerificationMethodMessage;
use hiero_sdk::Client;

pub fn prepare_update_did_csm(
    did: HederaDid,
    updates: Vec<DIDUpdateOperation>,
) -> Result<CsmBatchSigningRequest, DIDError> {
    prepare_update_did_csm_with_options(did, updates, CsmPrepareOptions::default())
}

pub fn prepare_update_did_csm_with_options(
    did: HederaDid,
    updates: Vec<DIDUpdateOperation>,
    options: CsmPrepareOptions,
) -> Result<CsmBatchSigningRequest, DIDError> {
    if updates.is_empty() {
        return Err(DIDError::InvalidArgument(
            "CSM update request must contain at least one operation".to_string(),
        ));
    }

    let did_string = did.to_string();
    let topic_id = did.topic_id.clone();
    let expected_public_key_bytes = public_key_from_did(&did)?;
    let mut requests = Vec::with_capacity(updates.len());

    for update in updates {
        requests.push(prepare_update_operation_csm(
            &did_string,
            &topic_id,
            update,
            expected_public_key_bytes.clone(),
            options.clone(),
        )?);
    }

    Ok(CsmBatchSigningRequest {
        version: crate::csm::CSM_STATE_VERSION,
        did: did_string,
        topic_id,
        operation: "update".to_string(),
        lifecycle_label: PAUSE_FOR_SIGNATURE_LABEL.to_string(),
        requests,
    })
}

pub async fn submit_update_did_csm(
    client: &Client,
    request: CsmBatchSubmitRequest,
) -> Result<CsmBatchSubmitResult, DIDError> {
    if request.requests.is_empty() {
        return Err(DIDError::InvalidArgument(
            "CSM update submit request must contain at least one signed operation".to_string(),
        ));
    }

    for item in &request.requests {
        if item.state.operation != "update" {
            return Err(DIDError::InvalidArgument(format!(
                "Expected update CSM state, got {}",
                item.state.operation
            )));
        }
    }

    submit_csm_batch(client, request).await
}

fn prepare_update_operation_csm(
    did: &str,
    topic_id: &str,
    update: DIDUpdateOperation,
    expected_public_key_bytes: Vec<u8>,
    options: CsmPrepareOptions,
) -> Result<CsmSigningRequest, DIDError> {
    match update {
        DIDUpdateOperation::AddVerificationMethod(opts) => {
            if matches!(
                opts.property,
                VerificationMethodProperty::VerificationMethod
            ) && opts.public_key_multibase.is_none()
            {
                return Err(DIDError::InvalidArgument(
                    "public_key_multibase is required for verificationMethod property".into(),
                ));
            }

            let controller = opts.controller.unwrap_or_else(|| did.to_string());
            let public_key_multibase = opts.public_key_multibase.unwrap_or_default();
            let property = opts.property.as_str().to_string();
            let message = DIDAddVerificationMethodMessage::new(
                did.to_string(),
                opts.id.clone(),
                property.clone(),
                controller.clone(),
                public_key_multibase.clone(),
            );

            build_state(
                did.to_string(),
                topic_id.to_string(),
                "update".to_string(),
                CsmMessageState::AddVerificationMethod {
                    did: did.to_string(),
                    id: opts.id,
                    property,
                    controller,
                    public_key_multibase,
                    timestamp: message.timestamp,
                },
                expected_public_key_bytes,
                options,
            )?
            .signing_request()
        }
        DIDUpdateOperation::RemoveVerificationMethod(opts) => {
            let message = DIDRemoveVerificationMethodMessage::new(did.to_string(), opts.id.clone());

            build_state(
                did.to_string(),
                topic_id.to_string(),
                "update".to_string(),
                CsmMessageState::RemoveVerificationMethod {
                    did: did.to_string(),
                    id: opts.id,
                    timestamp: message.timestamp,
                },
                expected_public_key_bytes,
                options,
            )?
            .signing_request()
        }
        DIDUpdateOperation::AddService(opts) => {
            let message = DIDAddServiceMessage::new(
                did.to_string(),
                opts.id.clone(),
                opts.service_type.clone(),
                opts.service_endpoint.clone(),
            );

            build_state(
                did.to_string(),
                topic_id.to_string(),
                "update".to_string(),
                CsmMessageState::AddService {
                    did: did.to_string(),
                    id: opts.id,
                    service_type: opts.service_type,
                    service_endpoint: opts.service_endpoint,
                    timestamp: message.timestamp,
                },
                expected_public_key_bytes,
                options,
            )?
            .signing_request()
        }
        DIDUpdateOperation::RemoveService(opts) => {
            let message = DIDRemoveServiceMessage::new(did.to_string(), opts.id.clone());

            build_state(
                did.to_string(),
                topic_id.to_string(),
                "update".to_string(),
                CsmMessageState::RemoveService {
                    did: did.to_string(),
                    id: opts.id,
                    timestamp: message.timestamp,
                },
                expected_public_key_bytes,
                options,
            )?
            .signing_request()
        }
    }
}
