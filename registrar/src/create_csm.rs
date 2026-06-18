use hiero_did_core::did::Network;
use hiero_did_core::{
    DIDError,
    HederaDid,
    KeysUtility,
};
use hiero_did_hcs::HcsTopic;
use hiero_did_lifecycle::{
    LifecycleBuilder,
    LifecycleRunner,
    LifecycleRunnerOptions,
    RunnerStatus,
};
use hiero_did_messages::DIDOwnerMessage;
use hiero_sdk::Client;

use crate::csm::{
    CsmMessageState,
    CsmOperationState,
    CsmPrepareOptions,
    CsmSigningRequest,
    CsmSubmitRequest,
    CsmSubmitResult,
    build_state,
    submit_csm_request,
};

const STEP_SIGN: &str = "pause-for-signature";

fn create_lifecycle() -> Result<LifecycleRunner<DIDOwnerMessage, CsmOperationState>, DIDError> {
    let builder = LifecycleBuilder::new().pause(STEP_SIGN)?;
    Ok(LifecycleRunner::new(builder))
}

pub async fn prepare_create_did_csm(
    client: &Client,
    network: Network,
    public_key_bytes: Vec<u8>,
    controller: Option<String>,
) -> Result<CsmSigningRequest, DIDError> {
    prepare_create_did_csm_with_options(
        client,
        network,
        public_key_bytes,
        controller,
        CsmPrepareOptions::default(),
    )
    .await
}

pub async fn prepare_create_did_csm_with_options(
    client: &Client,
    network: Network,
    public_key_bytes: Vec<u8>,
    controller: Option<String>,
    options: CsmPrepareOptions,
) -> Result<CsmSigningRequest, DIDError> {
    let topic_id = HcsTopic::create(client).await?;
    let topic_id_str = topic_id.to_string();
    let base58_key = KeysUtility::from_bytes(public_key_bytes.clone()).to_base58();
    let did = HederaDid::new(network, base58_key, topic_id_str.clone());
    let message = DIDOwnerMessage::new(did.clone(), public_key_bytes.clone(), controller.clone());
    let did_string = did.to_string();

    let csm_state = build_state(
        did_string.clone(),
        topic_id_str,
        "create".to_string(),
        CsmMessageState::DidOwner {
            did: did_string,
            public_key_bytes: public_key_bytes.clone(),
            timestamp: message.timestamp.clone(),
            controller,
        },
        public_key_bytes,
        options,
    )?;

    let runner = create_lifecycle()?;
    let runner_state = runner.process(message, LifecycleRunnerOptions::new(csm_state)).await?;

    if runner_state.status != RunnerStatus::Pause {
        return Err(DIDError::InternalError("Expected lifecycle to pause for signature".into()));
    }

    runner_state.context.signing_request()
}

pub async fn submit_create_did_csm(
    client: &Client,
    request: CsmSubmitRequest,
) -> Result<CsmSubmitResult, DIDError> {
    if request.state.operation != "create" {
        return Err(DIDError::InvalidArgument(format!(
            "Expected create CSM state, got {}",
            request.state.operation
        )));
    }

    submit_csm_request(client, request).await
}
