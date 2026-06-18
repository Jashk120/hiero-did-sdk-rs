use hiero_did_core::DIDError;
use hiero_did_lifecycle::LifecycleMessage;

use crate::{
    DIDAddServiceMessage,
    DIDAddVerificationMethodMessage,
    DIDRemoveServiceMessage,
    DIDRemoveVerificationMethodMessage,
};

#[derive(Debug, Clone)]
pub enum DIDUpdateMessage {
    AddVerificationMethod(DIDAddVerificationMethodMessage),
    RemoveVerificationMethod(DIDRemoveVerificationMethodMessage),
    AddService(DIDAddServiceMessage),
    RemoveService(DIDRemoveServiceMessage),
}

impl LifecycleMessage for DIDUpdateMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        match self {
            Self::AddVerificationMethod(m) => m.message_bytes(),
            Self::RemoveVerificationMethod(m) => m.message_bytes(),
            Self::AddService(m) => m.message_bytes(),
            Self::RemoveService(m) => m.message_bytes(),
        }
    }

    fn set_signature(&mut self, _signature: Vec<u8>) -> Result<(), DIDError> {
        // No-op: these message types carry no `signature` field.
        // Signing is applied at submit time via `to_payload(&self, signature: &[u8])`,
        // with the signature threaded through CsmSubmitRequest/state independently.
        Ok(())
    }
}
