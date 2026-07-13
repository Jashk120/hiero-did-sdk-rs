use crate::DIDError;

/// Abstraction over a signing key.
///
/// Covers both raw DID message signing (used by registrar/resolver via
/// `InternalSigner`) and Hedera transaction signing (used by hcs via
/// `LocalSigner`). Transaction signing itself stays explicit on the SDK
/// types — this trait only abstracts raw crypto.
pub trait Signer: Send + Sync {
    /// Raw Ed25519 public key bytes (32 bytes).
    fn public_key_bytes(&self) -> Vec<u8>;

    /// Sign arbitrary bytes, returning a 64-byte Ed25519 signature.
    fn sign_bytes(&self, message: &[u8]) -> Result<Vec<u8>, DIDError>;
}

/// Validates that a given Ed25519 signature is exactly 64 bytes long,
/// enforcing the `Signer` trait's documented contract.
pub fn validate_ed25519_signature_len(signature: &[u8]) -> Result<(), DIDError> {
    if signature.len() != 64 {
        return Err(DIDError::InvalidArgument(format!(
            "Signature must be 64 bytes for Ed25519, got {}",
            signature.len()
        )));
    }
    Ok(())
}
