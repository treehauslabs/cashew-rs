use serde::{Deserialize, Serialize};
use std::fmt;

use crate::cid_utils;
use crate::encryption::EncryptionInfo;
use crate::error::{CashewError, Result};
use crate::merkle::radix_node::RadixNode;

/// A CID-linked reference to a `RadixNode`, supporting lazy resolution and encryption.
#[derive(Clone, Debug)]
pub struct RadixHeader<V> {
    pub raw_cid: String,
    pub node: Option<Box<RadixNode<V>>>,
    pub encryption_info: Option<EncryptionInfo>,
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> RadixHeader<V> {
    /// Creates a lazy reference (CID only, no loaded node).
    pub fn lazy(raw_cid: String) -> Self {
        Self {
            raw_cid,
            node: None,
            encryption_info: None,
        }
    }

    /// Creates a header from an already-loaded node. Computes CID from the node's serialized form.
    pub fn from_node(node: RadixNode<V>) -> Result<Self> {
        let data = node.to_data()?;
        let cid = cid_utils::compute_cid(&data)?;
        Ok(Self {
            raw_cid: cid_utils::cid_to_string(&cid),
            node: Some(Box::new(node)),
            encryption_info: None,
        })
    }

    /// Creates a header with all fields specified.
    pub fn new(
        raw_cid: String,
        node: Option<RadixNode<V>>,
        encryption_info: Option<EncryptionInfo>,
    ) -> Self {
        Self {
            raw_cid,
            node: node.map(Box::new),
            encryption_info,
        }
    }

    /// Returns a reference to the loaded node, or error if not loaded.
    pub fn require_node(&self) -> Result<&RadixNode<V>> {
        self.node.as_deref().ok_or(CashewError::NodeNotAvailable)
    }

    /// Returns a new header with the node stripped (lazy reference only).
    pub fn removing_node(&self) -> Self {
        Self {
            raw_cid: self.raw_cid.clone(),
            node: None,
            encryption_info: self.encryption_info.clone(),
        }
    }
}

/// Serializable form for RadixHeader — only CID and encryption info are persisted.
#[derive(Serialize, Deserialize)]
struct SerializableRadixHeader {
    raw_cid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    encryption_info: Option<EncryptionInfo>,
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> Serialize for RadixHeader<V> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let s = SerializableRadixHeader {
            raw_cid: self.raw_cid.clone(),
            encryption_info: self.encryption_info.clone(),
        };
        s.serialize(serializer)
    }
}

impl<'de, V: Clone + fmt::Display + Serialize + serde::de::DeserializeOwned> Deserialize<'de>
    for RadixHeader<V>
{
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = SerializableRadixHeader::deserialize(deserializer)?;
        Ok(RadixHeader {
            raw_cid: s.raw_cid,
            node: None,
            encryption_info: s.encryption_info,
        })
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> fmt::Display
    for RadixHeader<V>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw_cid)
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> PartialEq for RadixHeader<V> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_cid == other.raw_cid
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> Eq for RadixHeader<V> {}
