use std::collections::HashMap;
use std::str::FromStr;
use crate::did::HederaDid;
use crate::error::DIDError;

pub struct HederaDidUrl {
    pub did: HederaDid,
    pub path: Option<String>,
    pub params: HashMap<String, String>,
    pub fragment: Option<String>,
}

impl FromStr for HederaDidUrl {
    type Err = DIDError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Step 1: split off fragment
       let (before_fragment, fragment) = match s.split_once('#') {
            Some((left, right)) => {
                if right.is_empty() {
                    return Err(DIDError::InvalidDid("Empty fragment".into()));
                }
                (left, Some(right.to_string()))
            }
            None => (s, None),
        };

        // Step 2: split off query
        let (before_query, query) = match before_fragment.split_once('?') {
            Some((left, right)) => (left, Some(right)),
            None => (before_fragment, None),
        };

        // Step 3: split DID from path
        let (did_str, path) = match before_query.split_once('/') {
            Some((left, right)) => (left, Some(format!("/{}", right))),
            None => (before_query, None),
        };

        // Step 4: parse query into HashMap
        let mut params = HashMap::new();
            if let Some(query_str) = query {
                for pair in query_str.split('&') {
                    match pair.split_once('=') {
                        Some((key, value)) => { params.insert(key.to_string(), value.to_string()); }
                        None => return Err(DIDError::InvalidDid(format!("Malformed query param: {}", pair)))
                    }
                }
            }

        // Step 5: parse the DID portion
        let did = did_str.parse::<HederaDid>()?;

        Ok(HederaDidUrl { did, path, params, fragment })
    }
}

#[cfg(test)]
mod tests {
    use super::HederaDidUrl;

    #[test]
    fn bare_did() {
        // 1. parse the input — 
        let url: HederaDidUrl = "did:hedera:testnet:abc123_0.0.1234"
            .parse()
            .expect("should parse");

        // 2. check each field is what you expect
        assert_eq!(url.fragment, None);   // no # in input, so fragment should be None
        assert_eq!(url.path, None);       // no / after topic id, so path should be None
        assert!(url.params.is_empty());   // no ? in input, so no query params
    }

    #[test]
    fn with_fragment() {
        let url: HederaDidUrl = "did:hedera:testnet:abc123_0.0.1234#key-1"
            .parse()
            .expect("should parse");

        assert_eq!(url.fragment, Some("key-1".to_string()));
        assert_eq!(url.path, None);
    }

    #[test]
    fn with_query() {
        let url: HederaDidUrl = "did:hedera:testnet:abc123_0.0.1234?service=LinkedDomains"
            .parse()
            .expect("should parse");

        assert!(url.params["service"] == "LinkedDomains");
    }

    #[test]
    fn with_path_and_fragment() {
        let url: HederaDidUrl= "did:hedera:testnet:abc123_0.0.1234/some/path#key-1"
            .parse()
            .expect("should parse");
        
        assert_eq!(url.fragment, Some("key-1".to_string()));
        assert_eq!(url.path, Some("/some/path".to_string()));
    }

    #[test]
    fn invalid_did_url() {
           
        assert!("not-a-did".parse::<HederaDidUrl>().is_err());
    }
}