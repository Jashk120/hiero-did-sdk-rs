use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonCredsSchema {
    #[serde(rename = "issuerId")]
    pub issuer_id: String,
    pub name: String,
    pub version: String,
    #[serde(rename = "attrNames")]
    pub attr_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonCredsCredentialDefinition {
    #[serde(rename = "issuerId")]
    pub issuer_id: String,
    #[serde(rename = "schemaId")]
    pub schema_id: String,
    #[serde(rename = "type")]
    pub cred_type: String, // "CL"
    pub tag: String,
    pub value: CredentialDefinitionValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialDefinitionValue {
    pub primary: HashMap<String, serde_json::Value>,
    pub revocation: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonCredsRevocationRegistryDefinition {
    #[serde(rename = "issuerId")]
    pub issuer_id: String,
    #[serde(rename = "revocDefType")]
    pub revoc_def_type: String, // "CL_ACCUM"
    #[serde(rename = "credDefId")]
    pub cred_def_id: String,
    pub tag: String,
    pub value: RevocationRegistryDefinitionValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationRegistryDefinitionValue {
    #[serde(rename = "publicKeys")]
    pub public_keys: RevocationRegistryPublicKeys,
    #[serde(rename = "maxCredNum")]
    pub max_cred_num: u32,
    #[serde(rename = "tailsLocation")]
    pub tails_location: String,
    #[serde(rename = "tailsHash")]
    pub tails_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationRegistryPublicKeys {
    #[serde(rename = "accumKey")]
    pub accum_key: AccumKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumKey {
    pub z: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonCredsRevocationRegistryDefinitionWithMetadata {
    #[serde(rename = "revRegDef")]
    pub rev_reg_def: AnonCredsRevocationRegistryDefinition,
    #[serde(rename = "hcsMetadata")]
    pub hcs_metadata: HcsMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcsMetadata {
    #[serde(rename = "entriesTopicId")]
    pub entries_topic_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonCredsRevocationStatusList {
    #[serde(rename = "issuerId")]
    pub issuer_id: String,
    #[serde(rename = "revRegDefId")]
    pub rev_reg_def_id: String,
    #[serde(rename = "revocationList")]
    pub revocation_list: Vec<u8>,
    #[serde(rename = "currentAccumulator")]
    pub current_accumulator: String,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn schema_serde_field_names_match_anoncreds_shape() {
        let schema = AnonCredsSchema {
            issuer_id: "did:hedera:testnet:abc_0.0.1".to_string(),
            name: "Employment".to_string(),
            version: "1.0".to_string(),
            attr_names: vec!["role".to_string(), "country".to_string()],
        };

        let value = serde_json::to_value(schema).expect("to value");
        assert!(value.get("issuerId").is_some());
        assert!(value.get("attrNames").is_some());
        assert!(value.get("issuer_id").is_none());
        assert!(value.get("attr_names").is_none());
    }

    #[test]
    fn cred_def_and_revocation_types_roundtrip_json() {
        let cred_def = AnonCredsCredentialDefinition {
            issuer_id: "did:hedera:testnet:abc_0.0.1".to_string(),
            schema_id: "did:hedera:testnet:abc_0.0.1/anoncreds/v1/SCHEMA/0.0.123".to_string(),
            cred_type: "CL".to_string(),
            tag: "tag-1".to_string(),
            value: CredentialDefinitionValue {
                primary: HashMap::new(),
                revocation: None,
            },
        };

        let encoded = serde_json::to_vec(&cred_def).expect("encode");
        let decoded: AnonCredsCredentialDefinition =
            serde_json::from_slice(&encoded).expect("decode");
        assert_eq!(decoded.cred_type, "CL");
        assert_eq!(decoded.tag, "tag-1");
    }
}
