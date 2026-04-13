use cid::Cid;
use multihash::Multihash;
use multihash_codetable::{Code, MultihashDigest};

use crate::error::{CashewError, Result};

/// dag-json multicodec code
const DAG_JSON: u64 = 0x0129;

/// Computes a CID v1 from serialized data using SHA-256 and dag-json codec.
pub fn compute_cid(data: &[u8]) -> Result<Cid> {
    let hash = Code::Sha2_256.digest(data);
    let mh = Multihash::from_bytes(&hash.to_bytes())
        .map_err(|e| CashewError::CidCreationFailed(e.to_string()))?;
    Ok(Cid::new_v1(DAG_JSON, mh))
}

/// Encodes a CID to its base-encoded string representation.
pub fn cid_to_string(cid: &Cid) -> String {
    cid.to_string()
}

/// Parses a CID from a string.
pub fn cid_from_string(s: &str) -> Result<Cid> {
    s.parse::<Cid>()
        .map_err(|e| CashewError::CidCreationFailed(e.to_string()))
}

/// Serializes a value to deterministic JSON bytes.
pub fn to_deterministic_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| CashewError::SerializationFailed(e.to_string()))
}

/// Deserializes from JSON bytes.
pub fn from_json<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T> {
    serde_json::from_slice(data).map_err(|e| CashewError::DecodeError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_compute_cid_deterministic() {
        let data = b"hello world";
        let cid1 = compute_cid(data).unwrap();
        let cid2 = compute_cid(data).unwrap();
        assert_eq!(cid1, cid2);
    }

    #[test]
    fn test_cid_string_roundtrip() {
        let data = b"test data";
        let cid = compute_cid(data).unwrap();
        let s = cid_to_string(&cid);
        let parsed = cid_from_string(&s).unwrap();
        assert_eq!(cid, parsed);
    }

    #[test]
    fn test_different_data_different_cids() {
        let cid1 = compute_cid(b"data1").unwrap();
        let cid2 = compute_cid(b"data2").unwrap();
        assert_ne!(cid1, cid2);
    }

    #[test]
    fn test_deterministic_json_sorted() {
        let mut map = BTreeMap::new();
        map.insert("b", "2");
        map.insert("a", "1");
        let json = to_deterministic_json(&map).unwrap();
        let s = String::from_utf8(json).unwrap();
        assert!(s.find("\"a\"").unwrap() < s.find("\"b\"").unwrap());
    }

    #[test]
    fn test_json_roundtrip() {
        let original = vec!["hello".to_string(), "world".to_string()];
        let data = to_deterministic_json(&original).unwrap();
        let restored: Vec<String> = from_json(&data).unwrap();
        assert_eq!(original, restored);
    }
}
