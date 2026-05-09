use hiero_did_core::HederaDid;

/// Returns true if the string is a valid did:hedera DID
pub fn is_hedera_did(s: &str) -> bool {
    s.parse::<HederaDid>().is_ok()
}

/// Returns true if the string is a valid Hedera topic ID (shard.realm.num)
pub fn is_topic_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| p.parse::<u64>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::{is_hedera_did, is_topic_id};

    #[test]
    fn validates_topic_id() {
        assert!(is_topic_id("0.0.123"));
        assert!(!is_topic_id("0.123"));
        assert!(!is_topic_id("0.0.x"));
    }

    #[test]
    fn validates_hedera_did() {
        assert!(is_hedera_did("did:hedera:testnet:abc_0.0.123"));
        assert!(!is_hedera_did("did:hedera:testnet:abc_0.0.bad"));
        assert!(!is_hedera_did("did:other:testnet:abc_0.0.123"));
    }
}
