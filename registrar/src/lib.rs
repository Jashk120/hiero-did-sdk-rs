pub mod create;
pub mod csm;
pub mod deactivate;
pub mod update;

pub use create::{
    CreateDIDResult,
    CreateDIDWithSignerResult,
    create_did,
    create_did_with_signer,
};
pub use csm::{
    CSM_STATE_VERSION,
    CsmBatchSigningRequest,
    CsmBatchSubmitRequest,
    CsmBatchSubmitResult,
    CsmMessageState,
    CsmOperationState,
    CsmPrepareOptions,
    CsmSignature,
    CsmSigningRequest,
    CsmSubmitRequest,
    CsmSubmitResult,
    PAUSE_BEFORE_PUBLISH_LABEL,
    PAUSE_FOR_SIGNATURE_LABEL,
    prepare_create_did_csm,
    prepare_create_did_csm_with_options,
    prepare_deactivate_did_csm,
    prepare_deactivate_did_csm_with_options,
    prepare_update_did_csm,
    prepare_update_did_csm_with_options,
    submit_create_did_csm,
    submit_deactivate_did_csm,
    submit_update_did_csm,
};
pub use deactivate::{
    DeactivateDIDResult,
    DeactivatedDIDDocument,
    deactivate_did,
    deactivate_did_with_signer,
};

pub use update::RemoveService;
pub use update::{
    AddService,
    AddVerificationMethod,
    DIDUpdateOperation,
    RemoveVerificationMethod,
    UpdateDIDResult,
    VerificationMethodProperty,
    update_did,
    update_did_with_signer,
};