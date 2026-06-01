use crate::csm::CsmMessageState;
use crate::csm::CsmPrepareOptions;
use crate::csm::CsmSigningRequest;
use crate::csm::CsmSubmitRequest;
use crate::csm::CsmSubmitResult;
use crate::csm::build_state;
use crate::csm::public_key_from_did;
use crate::csm::submit_csm_request;
use hiero_did_core::DIDError;
use hiero_did_core::HederaDid;
use hiero_did_messages::DIDDeactivateMessage;
use hiero_sdk::Client;

pub fn prepare_deactivate_did_csm(did: HederaDid) -> Result<CsmSigningRequest, DIDError> {
    prepare_deactivate_did_csm_with_options(did, CsmPrepareOptions::default())
}

pub fn prepare_deactivate_did_csm_with_options(
    did: HederaDid,
    options: CsmPrepareOptions,
) -> Result<CsmSigningRequest, DIDError> {
    let topic_id = did.topic_id.clone();
    let did_string = did.to_string();
    let expected_public_key_bytes = public_key_from_did(&did)?;
    let message = DIDDeactivateMessage::new(did);

    build_state(
        did_string.clone(),
        topic_id,
        "delete".to_string(),
        CsmMessageState::Deactivate {
            did: did_string,
            timestamp: message.timestamp,
        },
        expected_public_key_bytes,
        options,
    )?
    .signing_request()
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
