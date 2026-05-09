use crate::error::DIDError;

/// ED25519 multicodec prefix varint-encoded: 237 -> [0xed, 0x01]
pub const ED25519_MULTICODEC_PREFIX: [u8; 2] = [0xed, 0x01];
pub const MULTIBASE_BASE58BTC_PREFIX: char = 'z';

pub struct KeysUtility {
    bytes: Vec<u8>,
}

impl KeysUtility {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn from_base58(s: &str) -> Result<Self, DIDError> {
        bs58::decode(s)
            .into_vec()
            .map(|bytes| Self { bytes })
            .map_err(|e| DIDError::InvalidArgument(format!("Invalid base58: {}", e)))
    }

    pub fn from_multibase(s: &str) -> Result<Self, DIDError> {
        let mut chars = s.chars();
        let prefix = chars.next().ok_or_else(|| DIDError::InvalidMultibase("Empty string".into()))?;

        match prefix {
            'z' => {
                let encoded = &s[1..];
                let bytes = bs58::decode(encoded)
                    .into_vec()
                    .map_err(|e| DIDError::InvalidMultibase(format!("Invalid base58btc: {}", e)))?;

                // strip multicodec prefix if present
                if bytes.len() > 2 && bytes[0] == ED25519_MULTICODEC_PREFIX[0] && bytes[1] == ED25519_MULTICODEC_PREFIX[1] {
                    Ok(Self { bytes: bytes[2..].to_vec() })
                } else {
                    Ok(Self { bytes })
                }
            }
            _ => Err(DIDError::InvalidMultibase(format!("Unsupported multibase prefix: {}", prefix))),
        }
    }

    /// Raw 32-byte key -> base58btc (no multicodec prefix)
    /// Used for the key portion of the DID string itself
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.bytes).into_string()
    }

    /// Raw 32-byte key -> 'z' + base58btc([0xed, 0x01, ...key_bytes])
    /// Used for publicKeyMultibase in DIDOwner event and DID document
    pub fn to_multibase(&self) -> String {
        let mut prefixed = Vec::with_capacity(2 + self.bytes.len());
        prefixed.extend_from_slice(&ED25519_MULTICODEC_PREFIX);
        prefixed.extend_from_slice(&self.bytes);
        format!("{}{}", MULTIBASE_BASE58BTC_PREFIX, bs58::encode(&prefixed).into_string())
    }

    pub fn to_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::KeysUtility;

    #[test]
    fn base58_round_trip() {
        let raw = vec![1u8; 32];
        let encoded = KeysUtility::from_bytes(raw.clone()).to_base58();
        let decoded = KeysUtility::from_base58(&encoded).expect("valid base58");
        assert_eq!(decoded.to_bytes(), raw.as_slice());
    }

    #[test]
    fn multibase_round_trip() {
        let raw = vec![2u8; 32];
        let encoded = KeysUtility::from_bytes(raw.clone()).to_multibase();
        assert!(encoded.starts_with('z'));
        let decoded = KeysUtility::from_multibase(&encoded).expect("valid multibase");
        assert_eq!(decoded.to_bytes(), raw.as_slice());
    }

    #[test]
    fn multibase_invalid_prefix_and_empty() {
        assert!(KeysUtility::from_multibase("").is_err());
        assert!(KeysUtility::from_multibase("fabc").is_err());
    }
}
