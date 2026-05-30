use hiero_did_core::DIDError;

pub trait LifecycleMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError>;
    fn set_signature(&mut self, signature: Vec<u8>) -> Result<(), DIDError>;
}
