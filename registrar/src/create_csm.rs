use crate::csm::CsmMessageState;
use crate::csm::CsmPrepareOptions;
use crate::csm::CsmSigningRequest;
use crate::csm::CsmSubmitRequest;
use crate::csm::CsmSubmitResult;
use crate::csm::build_state;
use crate::csm::submit_csm_request;
use hiero_did_core::DIDError;
use hiero_did_core::HederaDid;
use hiero_did_core::KeysUtility;
use hiero_did_core::did::Network;
use hiero_did_hcs::HcsTopic;
use hiero_did_messages::DIDOwnerMessage;
use hiero_sdk::Client;

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

    build_state(
        did_string.clone(),
        topic_id_str,
        "create".to_string(),
        CsmMessageState::DidOwner {
            did: did_string,
            public_key_bytes: public_key_bytes.clone(),
            timestamp: message.timestamp,
            controller,
        },
        public_key_bytes,
        options,
    )?
    .signing_request()
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
