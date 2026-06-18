use hiero_did_core::{
    DIDError,
    HederaDid,
};
use hiero_did_lifecycle::{
    LifecycleBuilder,
    LifecycleRunner,
    LifecycleRunnerOptions,
    RunnerStatus,
};
use hiero_did_messages::DIDUpdateMessage;
use hiero_sdk::Client;

use crate::csm::{
    CsmBatchSigningRequest,
    CsmBatchSubmitRequest,
    CsmBatchSubmitResult,
    CsmMessageState,
    CsmPrepareOptions,
    CsmSigningRequest,
    PAUSE_FOR_SIGNATURE_LABEL,
    build_state,
    public_key_from_did,
    submit_csm_batch,
};
use crate::update::{
    DIDUpdateMessageExt,
    DIDUpdateOperation,
};

const STEP_SIGN: &str = "pause-for-signature";

fn update_lifecycle()
-> Result<LifecycleRunner<DIDUpdateMessage, crate::csm::CsmOperationState>, DIDError> {
    let builder = LifecycleBuilder::new().pause(STEP_SIGN)?;
    Ok(LifecycleRunner::new(builder))
}

pub async fn prepare_update_did_csm(
    did: HederaDid,
    updates: Vec<DIDUpdateOperation>,
) -> Result<CsmBatchSigningRequest, DIDError> {
    prepare_update_did_csm_with_options(did, updates, CsmPrepareOptions::default()).await
}

pub async fn prepare_update_did_csm_with_options(
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
        requests.push(
            prepare_update_operation_csm(
                &did_string,
                &topic_id,
                update,
                expected_public_key_bytes.clone(),
                options.clone(),
            )
            .await?,
        );
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

async fn prepare_update_operation_csm(
    did: &str,
    topic_id: &str,
    update: DIDUpdateOperation,
    expected_public_key_bytes: Vec<u8>,
    options: CsmPrepareOptions,
) -> Result<CsmSigningRequest, DIDError> {
    let (message, csm_message_state) = {
        let did_str = did.to_string();
        let message = DIDUpdateMessage::from_operation(&did_str, update)?;
        let state = match &message {
            DIDUpdateMessage::AddVerificationMethod(m) => CsmMessageState::AddVerificationMethod {
                did: m.did.clone(),
                id: m.id.clone(),
                property: m.property.clone(),
                controller: m.controller.clone(),
                public_key_multibase: m.public_key_multibase.clone(),
                timestamp: m.timestamp.clone(),
            },
            DIDUpdateMessage::RemoveVerificationMethod(m) => {
                CsmMessageState::RemoveVerificationMethod {
                    did: m.did.clone(),
                    id: m.id.clone(),
                    timestamp: m.timestamp.clone(),
                }
            }
            DIDUpdateMessage::AddService(m) => CsmMessageState::AddService {
                did: m.did.clone(),
                id: m.id.clone(),
                service_type: m.service_type.clone(),
                service_endpoint: m.service_endpoint.clone(),
                timestamp: m.timestamp.clone(),
            },
            DIDUpdateMessage::RemoveService(m) => CsmMessageState::RemoveService {
                did: m.did.clone(),
                id: m.id.clone(),
                timestamp: m.timestamp.clone(),
            },
        };
        (message, state)
    };

    let csm_state = build_state(
        did.to_string(),
        topic_id.to_string(),
        "update".to_string(),
        csm_message_state,
        expected_public_key_bytes,
        options,
    )?;

    let runner = update_lifecycle()?;
    let runner_state = runner.process(message, LifecycleRunnerOptions::new(csm_state)).await?;

    if runner_state.status != RunnerStatus::Pause {
        return Err(DIDError::InternalError("Expected lifecycle to pause for signature".into()));
    }

    runner_state.context.signing_request()
}
