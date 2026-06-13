use crate::csm::CsmMessageState;
use crate::csm::CsmOperationState;
use crate::csm::CsmPrepareOptions;
use crate::csm::CsmSigningRequest;
use crate::csm::CsmSubmitRequest;
use crate::csm::CsmSubmitResult;
use crate::csm::build_state;
use crate::csm::public_key_from_did;
use crate::csm::submit_csm_request;
use hiero_did_core::DIDError;
use hiero_did_core::HederaDid;
use hiero_did_lifecycle::LifecycleBuilder;
use hiero_did_lifecycle::LifecycleRunner;
use hiero_did_lifecycle::LifecycleRunnerOptions;
use hiero_did_lifecycle::RunnerStatus;
use hiero_did_messages::DIDDeactivateMessage;
use hiero_sdk::Client;

const STEP_SIGN: &str = "pause-for-signature";

fn deactivate_lifecycle() -> Result<LifecycleRunner<DIDDeactivateMessage, CsmOperationState>, DIDError> {
    let builder = LifecycleBuilder::new().pause(STEP_SIGN)?;
    Ok(LifecycleRunner::new(builder))
}

pub async fn prepare_deactivate_did_csm(did: HederaDid) -> Result<CsmSigningRequest, DIDError> {
    prepare_deactivate_did_csm_with_options(did, CsmPrepareOptions::default()).await
}

pub async fn prepare_deactivate_did_csm_with_options(
    did: HederaDid,
    options: CsmPrepareOptions,
) -> Result<CsmSigningRequest, DIDError> {
    let topic_id = did.topic_id.clone();
    let did_string = did.to_string();
    let expected_public_key_bytes = public_key_from_did(&did)?;
    let message = DIDDeactivateMessage::new(did);

    let csm_state = build_state(
        did_string.clone(),
        topic_id,
        "delete".to_string(),
        CsmMessageState::Deactivate {
            did: did_string,
            timestamp: message.timestamp.clone(),
        },
        expected_public_key_bytes,
        options,
    )?;

    let runner = deactivate_lifecycle()?;
    let runner_state = runner
        .process(message, LifecycleRunnerOptions::new(csm_state))
        .await?;

    if runner_state.status != RunnerStatus::Pause {
        return Err(DIDError::InternalError(
            "Expected lifecycle to pause for signature".into(),
        ));
    }

    runner_state.context.signing_request()
}

pub async fn submit_deactivate_did_csm(
    client: &Client,
    request: CsmSubmitRequest,
) -> Result<CsmSubmitResult, DIDError> {
    if request.state.operation != "delete" {
        return Err(DIDError::InvalidArgument(format!(
            "Expected delete CSM state, got {}",
            request.state.operation
        )));
    }

    submit_csm_request(client, request).await
}