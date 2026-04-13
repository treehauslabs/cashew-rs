use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

use crate::cid_utils;
use crate::error::Result;
use crate::merkle::radix_header::RadixHeader;

/// A sorted entry used for deterministic serialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(
    serialize = "V: serde::Serialize",
    deserialize = "V: serde::de::DeserializeOwned"
))]
pub struct SortedEntry<V> {
    pub key: String,
    pub value: V,
}

/// Internal node of a compressed radix trie in the Merkle tree.
///
/// Each node stores:
/// - A compressed `prefix` (edge label in the radix trie)
/// - An optional `value` at this node
/// - Child nodes keyed by the next character
#[derive(Clone, Debug)]
pub struct RadixNode<V> {
    pub prefix: String,
    pub value: Option<V>,
    pub children: BTreeMap<char, RadixHeader<V>>,
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> RadixNode<V> {
    pub fn new(prefix: String, value: Option<V>, children: BTreeMap<char, RadixHeader<V>>) -> Self {
        Self {
            prefix,
            value,
            children,
        }
    }

    pub fn empty() -> Self {
        Self::new(String::new(), None, BTreeMap::new())
    }

    pub fn leaf(prefix: String, value: V) -> Self {
        Self::new(prefix, Some(value), BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_none() && self.children.is_empty()
    }

    pub fn properties(&self) -> Vec<String> {
        self.children.keys().map(|c| c.to_string()).collect()
    }

    /// Serializes to deterministic JSON bytes for CID computation.
    pub fn to_data(&self) -> Result<Vec<u8>> {
        let serializable = SerializableRadixNode::from(self);
        cid_utils::to_deterministic_json(&serializable)
    }

    /// Deserializes from JSON bytes.
    pub fn from_data(data: &[u8]) -> Result<Self> {
        let serializable: SerializableRadixNode<V> = cid_utils::from_json(data)?;
        Ok(serializable.into())
    }
}

/// Serializable form that uses sorted entries for deterministic output.
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "V: Clone + std::fmt::Display + serde::Serialize + serde::de::DeserializeOwned",
    deserialize = "V: Clone + std::fmt::Display + serde::Serialize + serde::de::DeserializeOwned"
))]
struct SerializableRadixNode<V> {
    prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<V>,
    children: Vec<SortedEntry<RadixHeader<V>>>,
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> From<&RadixNode<V>>
    for SerializableRadixNode<V>
{
    fn from(node: &RadixNode<V>) -> Self {
        let children: Vec<SortedEntry<RadixHeader<V>>> = node
            .children
            .iter()
            .map(|(k, v)| SortedEntry {
                key: k.to_string(),
                value: v.clone(),
            })
            .collect();
        Self {
            prefix: node.prefix.clone(),
            value: node.value.clone(),
            children,
        }
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> From<SerializableRadixNode<V>>
    for RadixNode<V>
{
    fn from(s: SerializableRadixNode<V>) -> Self {
        let children: BTreeMap<char, RadixHeader<V>> = s
            .children
            .into_iter()
            .filter_map(|entry| entry.key.chars().next().map(|c| (c, entry.value)))
            .collect();
        RadixNode::new(s.prefix, s.value, children)
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> fmt::Display
    for RadixNode<V>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_data() {
            Ok(data) => write!(f, "{}", String::from_utf8_lossy(&data)),
            Err(_) => write!(
                f,
                "RadixNode(prefix={}, children={})",
                self.prefix,
                self.children.len()
            ),
        }
    }
}
