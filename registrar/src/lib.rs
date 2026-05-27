pub mod create;
pub mod deactivate;
pub mod update;

pub use create::{CreateDIDResult, CreateDIDWithSignerResult, create_did_with_signer};
pub use deactivate::{DeactivateDIDResult, DeactivatedDIDDocument, deactivate_did_with_signer};
pub use update::{
    AddService, AddVerificationMethod, DIDUpdateOperation, HcsSignable, RemoveService,
    RemoveVerificationMethod, UpdateDIDResult, VerificationMethodProperty, update_did_with_signer,
};
