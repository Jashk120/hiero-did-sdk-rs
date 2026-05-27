use std::sync::{Arc, Mutex};

use hiero_did_core::{DIDError, Signer};
use hiero_sdk::PublicKey;

pub(crate) type SigningErrorSlot = Arc<Mutex<Option<DIDError>>>;

pub(crate) fn signing_error_slot() -> SigningErrorSlot {
    Arc::new(Mutex::new(None))
}

pub(crate) fn public_key_from_signer(signer: &dyn Signer) -> Result<PublicKey, DIDError> {
    let bytes = signer.public_key_bytes();
    PublicKey::from_bytes_ed25519(&bytes)
        .map_err(|e| DIDError::InternalError(format!("Invalid public key from signer: {e}")))
}

pub(crate) fn sign_with_error_capture(
    signer: Arc<dyn Signer>,
    errors: SigningErrorSlot,
) -> impl Fn(&[u8]) -> Vec<u8> {
    move |bytes: &[u8]| match signer.sign_bytes(bytes) {
        Ok(sig) => sig,
        Err(err) => {
            if let Ok(mut slot) = errors.lock() {
                *slot = Some(err);
            }
            Vec::new()
        }
    }
}

pub(crate) fn take_signing_error(errors: &SigningErrorSlot) -> Result<(), DIDError> {
    let mut slot = errors
        .lock()
        .map_err(|_| DIDError::InternalError("signing error lock poisoned".into()))?;

    if let Some(err) = slot.take() {
        return Err(err);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{sign_with_error_capture, signing_error_slot, take_signing_error};
    use hiero_did_core::{DIDError, Signer};
    use std::sync::Arc;

    struct FailingSigner;

    impl Signer for FailingSigner {
        fn public_key_bytes(&self) -> Vec<u8> {
            vec![1u8; 32]
        }

        fn sign_bytes(&self, _message: &[u8]) -> Result<Vec<u8>, DIDError> {
            Err(DIDError::InternalError("vault unavailable".into()))
        }
    }

    #[test]
    fn captures_signing_errors_for_later_return() {
        let errors = signing_error_slot();
        let signer: Arc<dyn Signer> = Arc::new(FailingSigner);
        let sign = sign_with_error_capture(signer, Arc::clone(&errors));

        assert!(sign(b"payload").is_empty());

        let err = take_signing_error(&errors).expect_err("captured error");
        assert!(matches!(err, DIDError::InternalError(_)));
        assert!(take_signing_error(&errors).is_ok());
    }
}
