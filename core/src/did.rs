use crate::error::DIDError;

pub const DID_ROOT_KEY_ID: &str = "#did-root-key";
pub const DID_METHOD: &str = "hedera";

#[derive(Debug, Clone, PartialEq)]
pub enum Network {
    Mainnet,
    Testnet,
    Previewnet,
    Local,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Previewnet => write!(f, "previewnet"),
            Network::Local => write!(f, "local"),
        }
    }
}

impl std::str::FromStr for Network {
    type Err = DIDError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            "previewnet" => Ok(Network::Previewnet),
            "local" => Ok(Network::Local),
            _ => Err(DIDError::InvalidDid(format!("Unknown network: {}", s))),
        }
    }
}

/// Represents a parsed did:hedera DID
/// Format: did:hedera:<network>:<base58key>_<shard>.<realm>.<topicNum>
#[derive(Debug, Clone, PartialEq)]
pub struct HederaDid {
    pub network: Network,
    pub base58_key: String,
    pub topic_id: String,
}

impl HederaDid {
    pub fn new(network: Network, base58_key: String, topic_id: String) -> Self {
        Self { network, base58_key, topic_id }
    }

    pub fn to_did_string(&self) -> String {
        format!("did:hedera:{}:{}_{}", self.network, self.base58_key, self.topic_id)
    }

    pub fn root_key_id(&self) -> String {
        format!("{}{}", self.to_did_string(), DID_ROOT_KEY_ID)
    }
}

impl std::fmt::Display for HederaDid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_did_string())
    }
}

impl std::str::FromStr for HederaDid {
    type Err = DIDError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // did:hedera:<network>:<base58key>_<topicId>
        let parts: Vec<&str> = s.splitn(4, ':').collect();

        if parts.len() != 4 {
            return Err(DIDError::InvalidDid(format!(
                "Expected 4 colon-separated parts, got {}",
                s
            )));
        }

        if parts[0] != "did" || parts[1] != "hedera" {
            return Err(DIDError::InvalidDid(format!("Not a did:hedera DID: {}", s)));
        }

        let network: Network = parts[2].parse()?;

        // parts[3] is "<base58key>_<topicId>"
        let id_parts: Vec<&str> = parts[3].splitn(2, '_').collect();
        if id_parts.len() != 2 {
            return Err(DIDError::InvalidDid(format!(
                "Missing topic ID separator '_' in: {}",
                parts[3]
            )));
        }

        let base58_key = id_parts[0].to_string();
        let topic_id = id_parts[1].to_string();

        // basic topic ID validation: shard.realm.num
        let topic_parts: Vec<&str> = topic_id.split('.').collect();
        if topic_parts.len() != 3 || topic_parts.iter().any(|p| p.parse::<u64>().is_err()) {
            return Err(DIDError::InvalidDid(format!("Invalid topic ID: {}", topic_id)));
        }

        Ok(HederaDid { network, base58_key, topic_id })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DID_ROOT_KEY_ID,
        HederaDid,
        Network,
    };

    #[test]
    fn network_parse_and_display() {
        assert!(matches!("mainnet".parse::<Network>(), Ok(Network::Mainnet)));
        assert!(matches!("testnet".parse::<Network>(), Ok(Network::Testnet)));
        assert!(matches!("previewnet".parse::<Network>(), Ok(Network::Previewnet)));
        assert!(matches!("local".parse::<Network>(), Ok(Network::Local)));
        assert!("invalid".parse::<Network>().is_err());
        assert_eq!(Network::Mainnet.to_string(), "mainnet");
        assert_eq!(Network::Previewnet.to_string(), "previewnet");
        assert_eq!(Network::Local.to_string(), "local");
    }

    #[test]
    fn hedera_did_round_trip() {
        let did = HederaDid::new(Network::Testnet, "7Nf9Qabc".to_string(), "0.0.12345".to_string());
        let s = did.to_did_string();
        let parsed: HederaDid = s.parse().expect("must parse");
        assert_eq!(parsed, did);
        assert_eq!(parsed.root_key_id(), format!("{s}{DID_ROOT_KEY_ID}"));
    }

    #[test]
    fn hedera_did_invalid_cases() {
        assert!("did:hedera:testnet:abc".parse::<HederaDid>().is_err());
        assert!("did:other:testnet:abc_0.0.1".parse::<HederaDid>().is_err());
        assert!("did:hedera:unknown:abc_0.0.1".parse::<HederaDid>().is_err());
        assert!("did:hedera:testnet:abc_0.0.bad".parse::<HederaDid>().is_err());
    }
}
