use base64::Engine;
use hiero_did_core::{
    DIDError,
    HederaDid,
};

pub const ANONCREDS_SEPARATOR: &str = "/";
pub const ANONCREDS_OBJECT_FAMILY: &str = "anoncreds";
pub const ANONCREDS_VERSION: &str = "v1";

#[derive(Debug, Clone, PartialEq)]
pub enum AnonCredsObjectType {
    Schema,
    PublicCredDef,
    RevReg,
    RevRegEntry,
}

impl AnonCredsObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Schema => "SCHEMA",
            Self::PublicCredDef => "PUBLIC_CRED_DEF",
            Self::RevReg => "REV_REG",
            Self::RevRegEntry => "REV_REG_ENTRY",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnonCredsIdentifier {
    pub issuer_did: String,
    pub network_name: String,
    pub object_family: String,
    pub version: String,
    pub object_type: String,
    pub topic_id: String,
}

pub fn build_anoncreds_identifier(
    publisher_did: &str,
    topic_id: &str,
    object_type: AnonCredsObjectType,
) -> String {
    [publisher_did, ANONCREDS_OBJECT_FAMILY, ANONCREDS_VERSION, object_type.as_str(), topic_id]
        .join(ANONCREDS_SEPARATOR)
}

pub fn parse_anoncreds_identifier(id: &str) -> Result<AnonCredsIdentifier, DIDError> {
    let parts: Vec<&str> = id.splitn(5, ANONCREDS_SEPARATOR).collect();

    if parts.len() != 5 {
        return Err(DIDError::InvalidArgument(format!(
            "Invalid AnonCreds identifier, expected 5 parts: {id}"
        )));
    }

    let did: HederaDid = parts[0].parse().map_err(|_| {
        DIDError::InvalidArgument(format!("Invalid DID in identifier: {}", parts[0]))
    })?;

    Ok(AnonCredsIdentifier {
        issuer_did: parts[0].to_string(),
        network_name: did.network.to_string(),
        object_family: parts[1].to_string(),
        version: parts[2].to_string(),
        object_type: parts[3].to_string(),
        topic_id: parts[4].to_string(),
    })
}

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationRegistryEntryValue {
    pub accum: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_accum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationRegistryEntry {
    pub value: RevocationRegistryEntryValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationRegistryEntryWrapper {
    pub payload: String, // base64 encoded zstd compressed RevocationRegistryEntry
}

pub fn pack_revocation_entry(
    entry: &RevocationRegistryEntry,
) -> Result<String, hiero_did_core::DIDError> {
    let json = serde_json::to_vec(entry)
        .map_err(|e| hiero_did_core::DIDError::SerializationError(e.to_string()))?;
    let compressed = zstd::encode_all(json.as_slice(), 0).map_err(|e| {
        hiero_did_core::DIDError::InternalError(format!("zstd compress failed: {e}"))
    })?;
    let payload = base64::engine::general_purpose::STANDARD.encode(&compressed);
    let wrapper = RevocationRegistryEntryWrapper { payload };
    serde_json::to_string(&wrapper)
        .map_err(|e| hiero_did_core::DIDError::SerializationError(e.to_string()))
}

pub fn unpack_revocation_entry(data: &[u8]) -> Option<RevocationRegistryEntry> {
    let text = String::from_utf8(data.to_vec()).ok()?;
    let wrapper: RevocationRegistryEntryWrapper = serde_json::from_str(&text).ok()?;
    let compressed = base64::engine::general_purpose::STANDARD.decode(&wrapper.payload).ok()?;
    let json = zstd::decode_all(compressed.as_slice()).ok()?;
    serde_json::from_slice(&json).ok()
}

pub fn compute_status_list_diff(
    original: &[u8],
    modified: &[u8],
) -> Result<(Vec<u32>, Vec<u32>), hiero_did_core::DIDError> {
    if original.len() != modified.len() {
        return Err(hiero_did_core::DIDError::InvalidArgument(
            "Status lists must have the same length".into(),
        ));
    }
    let mut issued = vec![];
    let mut revoked = vec![];
    for (i, (o, m)) in original.iter().zip(modified.iter()).enumerate() {
        match (o, m) {
            (1, 0) => issued.push(i as u32),
            (0, 1) => revoked.push(i as u32),
            _ => {}
        }
    }
    Ok((issued, revoked))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_identifier_schema() {
        let did = "did:hedera:testnet:zFAeKMsqnNc2bwEsC8oqENBvGqjpGu9tpUi3VWaFEBXBo_0.0.5896419";
        let topic_id = "0.0.5896422";
        let id = build_anoncreds_identifier(did, topic_id, AnonCredsObjectType::Schema);
        assert_eq!(
            id,
            "did:hedera:testnet:zFAeKMsqnNc2bwEsC8oqENBvGqjpGu9tpUi3VWaFEBXBo_0.0.5896419/anoncreds/v1/SCHEMA/0.0.5896422"
        );
    }

    #[test]
    fn parse_identifier_roundtrip() {
        let id = "did:hedera:testnet:zFAeKMsqnNc2bwEsC8oqENBvGqjpGu9tpUi3VWaFEBXBo_0.0.5896419/anoncreds/v1/SCHEMA/0.0.5896422";
        let parsed = parse_anoncreds_identifier(id).expect("should parse");
        assert_eq!(
            parsed.issuer_did,
            "did:hedera:testnet:zFAeKMsqnNc2bwEsC8oqENBvGqjpGu9tpUi3VWaFEBXBo_0.0.5896419"
        );
        assert_eq!(parsed.network_name, "testnet");
        assert_eq!(parsed.object_family, "anoncreds");
        assert_eq!(parsed.version, "v1");
        assert_eq!(parsed.object_type, "SCHEMA");
        assert_eq!(parsed.topic_id, "0.0.5896422");
    }

    #[test]
    fn parse_identifier_invalid() {
        assert!(parse_anoncreds_identifier("not/valid").is_err());
        assert!(
            parse_anoncreds_identifier("did:hedera:testnet:abc_0.0.1/anoncreds/v1/SCHEMA").is_err()
        );
    }

    #[test]
    fn pack_and_unpack_revocation_entry_roundtrip() {
        let entry = RevocationRegistryEntry {
            value: RevocationRegistryEntryValue {
                accum: "accum-1".to_string(),
                prev_accum: Some("accum-0".to_string()),
                issued: Some(vec![1, 7, 9]),
                revoked: Some(vec![2, 8]),
            },
        };

        let packed = pack_revocation_entry(&entry).expect("pack should succeed");
        let unpacked = unpack_revocation_entry(packed.as_bytes()).expect("unpack should succeed");
        assert_eq!(unpacked.value.accum, "accum-1");
        assert_eq!(unpacked.value.prev_accum.as_deref(), Some("accum-0"));
        assert_eq!(unpacked.value.issued, Some(vec![1, 7, 9]));
        assert_eq!(unpacked.value.revoked, Some(vec![2, 8]));
    }

    #[test]
    fn compute_status_list_diff_detects_issued_and_revoked() {
        let original = vec![0, 1, 0, 1, 0];
        let modified = vec![1, 0, 0, 1, 1];
        let (issued, revoked) = compute_status_list_diff(&original, &modified).expect("diff");
        assert_eq!(issued, vec![1]);
        assert_eq!(revoked, vec![0, 4]);
    }

    #[test]
    fn compute_status_list_diff_rejects_length_mismatch() {
        let err = compute_status_list_diff(&[0, 1], &[0]).expect_err("must fail");
        assert!(err.to_string().contains("same length"));
    }

    #[test]
    fn wrapper_json_shape_is_stable() {
        let entry = RevocationRegistryEntry {
            value: RevocationRegistryEntryValue {
                accum: "abc".to_string(),
                prev_accum: None,
                issued: None,
                revoked: Some(vec![3]),
            },
        };
        let packed = pack_revocation_entry(&entry).expect("pack");
        let parsed: serde_json::Value = serde_json::from_str(&packed).expect("json");
        assert!(parsed.get("payload").and_then(|v| v.as_str()).is_some());
    }
}
