use hiero_did_core::{
    DIDError,
    HederaDid,
};

/// Parse a did:hedera string into a HederaDid
pub fn parse_did(did: &str) -> Result<HederaDid, DIDError> {
    did.parse()
}

#[cfg(test)]
mod tests {
    use super::parse_did;

    #[test]
    fn parse_valid_did() {
        let did = parse_did("did:hedera:testnet:abc_0.0.555").expect("must parse");
        assert_eq!(did.topic_id, "0.0.555");
    }

    #[test]
    fn reject_invalid_did() {
        assert!(parse_did("did:hedera:testnet:abc").is_err());
    }
}
