use serde::{Deserialize, Serialize};
use std::fmt;

use crate::cid_utils;
use crate::encryption::EncryptionInfo;
use crate::error::{CashewError, Result};

/// A generic CID-linked reference to any node type.
///
/// Similar to `RadixHeader` but works with any serializable node type,
/// including `MerkleDictionary`, `MerkleArray`, etc.
#[derive(Clone, Debug)]
pub struct Header<N> {
    pub raw_cid: String,
    pub node: Option<Box<N>>,
    pub encryption_info: Option<EncryptionInfo>,
}

impl<N: Clone + Serialize + for<'de> Deserialize<'de>> Header<N> {
    /// Creates a lazy reference (CID only).
    pub fn lazy(raw_cid: String) -> Self {
        Self {
            raw_cid,
            node: None,
            encryption_info: None,
        }
    }

    /// Creates a header from a node, computing its CID.
    pub fn from_node(node: N) -> Result<Self>
    where
        N: Serialize,
    {
        let data = cid_utils::to_deterministic_json(&node)?;
        let cid = cid_utils::compute_cid(&data)?;
        Ok(Self {
            raw_cid: cid_utils::cid_to_string(&cid),
            node: Some(Box::new(node)),
            encryption_info: None,
        })
    }

    /// Creates a header with all fields specified.
    pub fn new(raw_cid: String, node: Option<N>, encryption_info: Option<EncryptionInfo>) -> Self {
        Self {
            raw_cid,
            node: node.map(Box::new),
            encryption_info,
        }
    }

    pub fn require_node(&self) -> Result<&N> {
        self.node.as_deref().ok_or(CashewError::NodeNotAvailable)
    }

    pub fn removing_node(&self) -> Self {
        Self {
            raw_cid: self.raw_cid.clone(),
            node: None,
            encryption_info: self.encryption_info.clone(),
        }
    }
}

impl<N: Clone + Serialize + for<'de> Deserialize<'de>> fmt::Display for Header<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw_cid)
    }
}

impl<N: Clone + Serialize + for<'de> Deserialize<'de>> PartialEq for Header<N> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_cid == other.raw_cid
    }
}

impl<N: Clone + Serialize + for<'de> Deserialize<'de>> Eq for Header<N> {}

#[derive(Serialize, Deserialize)]
struct SerializableHeader {
    raw_cid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    encryption_info: Option<EncryptionInfo>,
}

impl<N: Clone + Serialize + for<'de> Deserialize<'de>> Serialize for Header<N> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let s = SerializableHeader {
            raw_cid: self.raw_cid.clone(),
            encryption_info: self.encryption_info.clone(),
        };
        s.serialize(serializer)
    }
}

impl<'de, N: Clone + Serialize + serde::de::DeserializeOwned> Deserialize<'de> for Header<N> {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = SerializableHeader::deserialize(deserializer)?;
        Ok(Header {
            raw_cid: s.raw_cid,
            node: None,
            encryption_info: s.encryption_info,
        })
    }
}
