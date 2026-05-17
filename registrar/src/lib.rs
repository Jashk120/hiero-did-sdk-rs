pub mod create;
pub mod update;
pub mod deactivate;

pub use create::CreateDIDResult;
pub use update::{
    UpdateDIDResult,
    DIDUpdateOperation,
    AddVerificationMethod,
    RemoveVerificationMethod,
    AddService,
    RemoveService,
    VerificationMethodProperty,
    HcsSignable,
};
pub use deactivate::{DeactivateDIDResult, DeactivatedDIDDocument};