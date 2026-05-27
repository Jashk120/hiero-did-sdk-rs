use ed25519_dalek::{Signature, Signer as DalekSigner, SigningKey, Verifier, VerifyingKey};
use hiero_did_core::{DIDError, Signer};

pub struct InternalSigner {
    signing_key: SigningKey,
}

impl InternalSigner {
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, DIDError> {
        Ok(Self {
            signing_key: SigningKey::from_bytes(bytes),
        })
    }

    pub fn from_raw_bytes(bytes: &[u8]) -> Result<Self, DIDError> {
        let arr: &[u8; 32] = bytes
            .try_into()
            .map_err(|_| DIDError::InvalidArgument("Private key must be 32 bytes".into()))?;
        Self::from_bytes(arr)
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    pub fn verifying_key_bytes(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }
}

impl Signer for InternalSigner {
    fn public_key_bytes(&self) -> Vec<u8> {
        self.verifying_key_bytes()
    }

    fn sign_bytes(&self, message: &[u8]) -> Result<Vec<u8>, DIDError> {
        Ok(self.sign(message))
    }
}

pub struct InternalVerifier {
    verifying_key: VerifyingKey,
}

impl InternalVerifier {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DIDError> {
        let arr: &[u8; 32] = bytes
            .try_into()
            .map_err(|_| DIDError::InvalidArgument("Public key must be 32 bytes".into()))?;
        Ok(Self {
            verifying_key: VerifyingKey::from_bytes(arr)
                .map_err(|e| DIDError::InvalidArgument(format!("Invalid public key: {}", e)))?,
        })
    }

    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, DIDError> {
        let sig_arr: &[u8; 64] = signature
            .try_into()
            .map_err(|_| DIDError::InvalidSignature("Signature must be 64 bytes".into()))?;
        let sig = Signature::from_bytes(sig_arr);
        Ok(self.verifying_key.verify(message, &sig).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::{InternalSigner, InternalVerifier};
    use hiero_did_core::Signer;

    #[test]
    fn sign_and_verify_happy_path() {
        let key = [7u8; 32];
        let signer = InternalSigner::from_bytes(&key).expect("valid signer");
        let verifier =
            InternalVerifier::from_bytes(&signer.verifying_key_bytes()).expect("valid verifier");
        let msg = b"hello-did";
        let sig = signer.sign_bytes(msg).expect("sign must succeed");
        assert!(
            verifier
                .verify(msg, &sig)
                .expect("verification must succeed")
        );
    }

    #[test]
    fn verify_fails_for_wrong_message() {
        let key = [9u8; 32];
        let signer = InternalSigner::from_bytes(&key).expect("valid signer");
        let verifier =
            InternalVerifier::from_bytes(&signer.verifying_key_bytes()).expect("valid verifier");
        let sig = signer.sign_bytes(b"msg-a").expect("sign must succeed");
        assert!(
            !verifier
                .verify(b"msg-b", &sig)
                .expect("verification must run")
        );
    }

    #[test]
    fn invalid_lengths_are_rejected() {
        assert!(InternalSigner::from_raw_bytes(&[1u8; 31]).is_err());
        assert!(InternalVerifier::from_bytes(&[2u8; 31]).is_err());
    }

    #[test]
    fn signs_through_signer_trait_object() {
        let key = [11u8; 32];
        let signer = InternalSigner::from_bytes(&key).expect("valid signer");
        let signer_ref: &dyn hiero_did_core::Signer = &signer;

        let sig = signer_ref
            .sign_bytes(b"trait-object-message")
            .expect("sign through trait");

        assert_eq!(signer_ref.public_key_bytes().len(), 32);
        assert_eq!(sig.len(), 64);
    }
}
