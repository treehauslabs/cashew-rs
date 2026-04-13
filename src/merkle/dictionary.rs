use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

use crate::cid_utils;
use crate::diff::{CashewDiff, ModifiedEntry};
use crate::error::{CashewError, Result};
use crate::merkle::radix_header::RadixHeader;
use crate::merkle::radix_node::{RadixNode, SortedEntry};
use crate::transform::Transform;

/// A persistent key-value store backed by a content-addressed compressed radix trie.
///
/// All mutations return new instances — the original remains unchanged.
/// CIDs are computed from deterministically serialized JSON.
///
/// # Examples
/// ```
/// use cashew::MerkleDictionary;
///
/// let dict = MerkleDictionary::<String>::new();
/// let dict = dict.inserting("alice", "engineer".to_string()).unwrap();
/// let dict = dict.inserting("bob", "designer".to_string()).unwrap();
///
/// assert_eq!(dict.get("alice").unwrap(), Some("engineer".to_string()));
/// assert_eq!(dict.len(), 2);
/// ```
#[derive(Clone, Debug)]
pub struct MerkleDictionary<V> {
    pub children: BTreeMap<char, RadixHeader<V>>,
    pub count: usize,
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> MerkleDictionary<V> {
    /// Creates an empty dictionary.
    pub fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            count: 0,
        }
    }

    /// Returns the number of key-value pairs.
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the value for a key by traversing the radix trie.
    pub fn get(&self, key: &str) -> Result<Option<V>> {
        if key.is_empty() {
            return Ok(None);
        }
        let first = key.chars().next().unwrap();
        let header = match self.children.get(&first) {
            Some(h) => h,
            None => return Ok(None),
        };
        let node = header.require_node()?;
        radix_get(node, key)
    }

    /// Returns a new dictionary with the key-value pair inserted.
    pub fn inserting(&self, key: &str, value: V) -> Result<Self>
    where
        V: fmt::Display,
    {
        if key.is_empty() {
            return Err(CashewError::InvalidKey("empty key".into()));
        }
        let first = key.chars().next().unwrap();
        let new_node = match self.children.get(&first) {
            Some(header) => {
                let node = header.require_node()?;
                radix_insert(node, key, value)?
            }
            None => RadixNode::leaf(key.to_string(), value),
        };
        let new_header = RadixHeader::from_node(new_node)?;
        let mut new_children = self.children.clone();
        new_children.insert(first, new_header);
        Ok(Self {
            children: new_children,
            count: self.count + 1,
        })
    }

    /// Returns a new dictionary with the value at key updated.
    pub fn mutating(&self, key: &str, value: V) -> Result<Self>
    where
        V: fmt::Display,
    {
        if key.is_empty() {
            return Err(CashewError::InvalidKey("empty key".into()));
        }
        let first = key.chars().next().unwrap();
        let header = self
            .children
            .get(&first)
            .ok_or_else(|| CashewError::InvalidKey(format!("key not found: {}", key)))?;
        let node = header.require_node()?;
        let new_node = radix_update(node, key, value)?;
        let new_header = RadixHeader::from_node(new_node)?;
        let mut new_children = self.children.clone();
        new_children.insert(first, new_header);
        Ok(Self {
            children: new_children,
            count: self.count,
        })
    }

    /// Returns a new dictionary with the key removed.
    pub fn deleting(&self, key: &str) -> Result<Self> {
        if key.is_empty() {
            return Err(CashewError::InvalidKey("empty key".into()));
        }
        let first = key.chars().next().unwrap();
        let header = match self.children.get(&first) {
            Some(h) => h,
            None => return Ok(self.clone()),
        };
        let node = header.require_node()?;
        let new_node = radix_delete(node, key)?;
        let mut new_children = self.children.clone();
        match new_node {
            Some(n) => {
                let new_header = RadixHeader::from_node(n)?;
                new_children.insert(first, new_header);
            }
            None => {
                new_children.remove(&first);
            }
        }
        Ok(Self {
            children: new_children,
            count: self.count.saturating_sub(1),
        })
    }

    /// Returns all keys in the dictionary.
    pub fn all_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        for header in self.children.values() {
            let node = header.require_node()?;
            collect_keys(node, "", &mut keys);
        }
        Ok(keys)
    }

    /// Returns all keys sorted, with optional pagination.
    pub fn sorted_keys(&self, limit: usize, after: Option<&str>) -> Result<Vec<String>> {
        let mut keys = self.all_keys()?;
        keys.sort();
        if let Some(cursor) = after {
            keys.retain(|k| k.as_str() > cursor);
        }
        keys.truncate(limit);
        Ok(keys)
    }

    /// Returns all key-value pairs.
    pub fn all_keys_and_values(&self) -> Result<Vec<(String, V)>> {
        let mut pairs = Vec::new();
        for header in self.children.values() {
            let node = header.require_node()?;
            collect_pairs(node, "", &mut pairs);
        }
        Ok(pairs)
    }

    /// Returns sorted key-value pairs with optional pagination.
    pub fn sorted_keys_and_values(
        &self,
        limit: usize,
        after: Option<&str>,
    ) -> Result<Vec<(String, V)>> {
        let mut pairs = self.all_keys_and_values()?;
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        if let Some(cursor) = after {
            pairs.retain(|(k, _)| k.as_str() > cursor);
        }
        pairs.truncate(limit);
        Ok(pairs)
    }

    /// Serializes to deterministic JSON for CID computation.
    pub fn to_data(&self) -> Result<Vec<u8>> {
        let sorted: Vec<SortedEntry<RadixHeader<V>>> = self
            .children
            .iter()
            .map(|(k, v)| SortedEntry {
                key: k.to_string(),
                value: v.clone(),
            })
            .collect();

        #[derive(Serialize)]
        #[serde(bound(serialize = "V: serde::Serialize"))]
        struct Serializable<V> {
            children: Vec<SortedEntry<V>>,
            count: usize,
        }

        cid_utils::to_deterministic_json(&Serializable {
            children: sorted,
            count: self.count,
        })
    }

    /// Deserializes from JSON bytes.
    pub fn from_data(data: &[u8]) -> Result<Self> {
        #[derive(Deserialize)]
        #[serde(bound(deserialize = "V: serde::de::DeserializeOwned"))]
        struct Deserializable<V> {
            children: Vec<SortedEntry<V>>,
            count: usize,
        }

        let d: Deserializable<RadixHeader<V>> = cid_utils::from_json(data)?;
        let children: BTreeMap<char, RadixHeader<V>> = d
            .children
            .into_iter()
            .filter_map(|entry| entry.key.chars().next().map(|c| (c, entry.value)))
            .collect();
        Ok(Self {
            children,
            count: d.count,
        })
    }

    /// Computes the diff between this dictionary and an older version.
    pub fn diff(&self, old: &Self) -> Result<CashewDiff>
    where
        V: PartialEq,
    {
        let new_keys = self.all_keys()?;
        let old_keys = old.all_keys()?;

        let new_set: std::collections::BTreeSet<String> = new_keys.iter().cloned().collect();
        let old_set: std::collections::BTreeSet<String> = old_keys.iter().cloned().collect();

        let mut diff = CashewDiff::new();

        for key in new_set.difference(&old_set) {
            if let Some(v) = self.get(key)? {
                diff.inserted.insert(key.clone(), v.to_string());
            }
        }

        for key in old_set.difference(&new_set) {
            if let Some(v) = old.get(key)? {
                diff.deleted.insert(key.clone(), v.to_string());
            }
        }

        for key in new_set.intersection(&old_set) {
            let new_val = self.get(key)?;
            let old_val = old.get(key)?;
            if new_val != old_val {
                diff.modified.insert(
                    key.clone(),
                    ModifiedEntry {
                        old: old_val.map(|v| v.to_string()).unwrap_or_default(),
                        new: new_val.map(|v| v.to_string()).unwrap_or_default(),
                        children: CashewDiff::new(),
                    },
                );
            }
        }

        Ok(diff)
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> Default
    for MerkleDictionary<V>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> fmt::Display
    for MerkleDictionary<V>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MerkleDictionary(count={})", self.count)
    }
}

// V needs From<String> for transform support
impl<V> MerkleDictionary<V>
where
    V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de> + From<String>,
{
    /// Applies a batch of transforms to produce a new dictionary.
    pub fn apply_transforms(&self, transforms: &array_trie::ArrayTrie<Transform>) -> Result<Self> {
        let mut result = self.clone();
        let keys = transforms.child_keys();
        for key in &keys {
            if let Some(transform) = transforms.get(&[key.as_str()]) {
                result = result.apply_transform(key, transform)?;
            }
        }
        Ok(result)
    }

    /// Applies a transform by key name and transform operation.
    pub fn apply_transform(&self, key: &str, transform: &Transform) -> Result<Self> {
        match transform {
            Transform::Insert(v) => self.inserting(key, V::from(v.clone())),
            Transform::Update(v) => self.mutating(key, V::from(v.clone())),
            Transform::Delete => self.deleting(key),
        }
    }
}

// ---- Radix trie operations ----

fn radix_get<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>>(
    node: &RadixNode<V>,
    key: &str,
) -> Result<Option<V>> {
    let key_chars: Vec<char> = key.chars().collect();
    let prefix_chars: Vec<char> = node.prefix.chars().collect();

    let mut ki = 0;
    let mut pi = 0;
    while ki < key_chars.len() && pi < prefix_chars.len() {
        if key_chars[ki] != prefix_chars[pi] {
            return Ok(None);
        }
        ki += 1;
        pi += 1;
    }

    if pi < prefix_chars.len() {
        return Ok(None);
    }

    if ki == key_chars.len() {
        return Ok(node.value.clone());
    }

    let next_char = key_chars[ki];
    match node.children.get(&next_char) {
        Some(header) => {
            let child = header.require_node()?;
            let remaining: String = key_chars[ki..].iter().collect();
            radix_get(child, &remaining)
        }
        None => Ok(None),
    }
}

fn radix_insert<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>>(
    node: &RadixNode<V>,
    key: &str,
    value: V,
) -> Result<RadixNode<V>> {
    let key_chars: Vec<char> = key.chars().collect();
    let prefix_chars: Vec<char> = node.prefix.chars().collect();

    let mut common = 0;
    while common < key_chars.len() && common < prefix_chars.len() {
        if key_chars[common] != prefix_chars[common] {
            break;
        }
        common += 1;
    }

    let key_done = common == key_chars.len();
    let prefix_done = common == prefix_chars.len();

    if key_done && prefix_done {
        return Ok(RadixNode::new(
            node.prefix.clone(),
            Some(value),
            node.children.clone(),
        ));
    }

    if prefix_done {
        let next_char = key_chars[common];
        let remaining: String = key_chars[common..].iter().collect();
        let new_children = if let Some(child_header) = node.children.get(&next_char) {
            let child = child_header.require_node()?;
            let new_child = radix_insert(child, &remaining, value)?;
            let new_header = RadixHeader::from_node(new_child)?;
            let mut c = node.children.clone();
            c.insert(next_char, new_header);
            c
        } else {
            let new_child = RadixNode::leaf(remaining, value);
            let new_header = RadixHeader::from_node(new_child)?;
            let mut c = node.children.clone();
            c.insert(next_char, new_header);
            c
        };
        return Ok(RadixNode::new(
            node.prefix.clone(),
            node.value.clone(),
            new_children,
        ));
    }

    if key_done {
        let remaining_prefix: String = prefix_chars[common..].iter().collect();
        let remaining_first = remaining_prefix.chars().next().unwrap();
        let existing = RadixNode::new(remaining_prefix, node.value.clone(), node.children.clone());
        let existing_header = RadixHeader::from_node(existing)?;
        let mut new_children = BTreeMap::new();
        new_children.insert(remaining_first, existing_header);
        let new_prefix: String = key_chars.iter().collect();
        return Ok(RadixNode::new(new_prefix, Some(value), new_children));
    }

    // Divergent
    let common_prefix: String = prefix_chars[..common].iter().collect();
    let key_remainder: String = key_chars[common..].iter().collect();
    let prefix_remainder: String = prefix_chars[common..].iter().collect();

    let key_first = key_remainder.chars().next().unwrap();
    let prefix_first = prefix_remainder.chars().next().unwrap();

    let existing = RadixNode::new(prefix_remainder, node.value.clone(), node.children.clone());
    let new_leaf = RadixNode::leaf(key_remainder, value);

    let existing_header = RadixHeader::from_node(existing)?;
    let new_header = RadixHeader::from_node(new_leaf)?;

    let mut new_children = BTreeMap::new();
    new_children.insert(prefix_first, existing_header);
    new_children.insert(key_first, new_header);

    Ok(RadixNode::new(common_prefix, None, new_children))
}

fn radix_update<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>>(
    node: &RadixNode<V>,
    key: &str,
    value: V,
) -> Result<RadixNode<V>> {
    // Same logic as insert but doesn't create new nodes
    radix_insert(node, key, value)
}

fn radix_delete<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>>(
    node: &RadixNode<V>,
    key: &str,
) -> Result<Option<RadixNode<V>>> {
    let key_chars: Vec<char> = key.chars().collect();
    let prefix_chars: Vec<char> = node.prefix.chars().collect();

    let mut common = 0;
    while common < key_chars.len() && common < prefix_chars.len() {
        if key_chars[common] != prefix_chars[common] {
            break;
        }
        common += 1;
    }

    let key_done = common == key_chars.len();
    let prefix_done = common == prefix_chars.len();

    if key_done && prefix_done {
        // Remove value at this node
        if node.children.is_empty() {
            return Ok(None);
        }
        if node.children.len() == 1 {
            let (_ch, child_header) = node.children.iter().next().unwrap();
            let child = child_header.require_node()?;
            let merged_prefix = format!("{}{}", node.prefix, child.prefix);
            return Ok(Some(RadixNode::new(
                merged_prefix,
                child.value.clone(),
                child.children.clone(),
            )));
        }
        return Ok(Some(RadixNode::new(
            node.prefix.clone(),
            None,
            node.children.clone(),
        )));
    }

    if prefix_done {
        let next_char = key_chars[common];
        let remaining: String = key_chars[common..].iter().collect();
        if let Some(child_header) = node.children.get(&next_char) {
            let child = child_header.require_node()?;
            match radix_delete(child, &remaining)? {
                Some(new_child) => {
                    let new_header = RadixHeader::from_node(new_child)?;
                    let mut new_children = node.children.clone();
                    new_children.insert(next_char, new_header);
                    return Ok(Some(RadixNode::new(
                        node.prefix.clone(),
                        node.value.clone(),
                        new_children,
                    )));
                }
                None => {
                    let mut new_children = node.children.clone();
                    new_children.remove(&next_char);
                    if new_children.is_empty() && node.value.is_none() {
                        return Ok(None);
                    }
                    if new_children.len() == 1 && node.value.is_none() {
                        let (_, only_header) = new_children.iter().next().unwrap();
                        let only_child = only_header.require_node()?;
                        let merged_prefix = format!("{}{}", node.prefix, only_child.prefix);
                        return Ok(Some(RadixNode::new(
                            merged_prefix,
                            only_child.value.clone(),
                            only_child.children.clone(),
                        )));
                    }
                    return Ok(Some(RadixNode::new(
                        node.prefix.clone(),
                        node.value.clone(),
                        new_children,
                    )));
                }
            }
        }
    }

    // Key doesn't match — nothing to delete
    Ok(Some(node.clone()))
}

fn collect_keys<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>>(
    node: &RadixNode<V>,
    prefix: &str,
    keys: &mut Vec<String>,
) {
    let full = format!("{}{}", prefix, node.prefix);
    if node.value.is_some() {
        keys.push(full.clone());
    }
    for header in node.children.values() {
        if let Some(ref child) = header.node {
            collect_keys(child, &full, keys);
        }
    }
}

fn collect_pairs<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>>(
    node: &RadixNode<V>,
    prefix: &str,
    pairs: &mut Vec<(String, V)>,
) {
    let full = format!("{}{}", prefix, node.prefix);
    if let Some(ref v) = node.value {
        pairs.push((full.clone(), v.clone()));
    }
    for header in node.children.values() {
        if let Some(ref child) = header.node {
            collect_pairs(child, &full, pairs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dictionary_is_empty() {
        let dict = MerkleDictionary::<String>::new();
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("alice", "engineer".to_string()).unwrap();
        assert_eq!(dict.get("alice").unwrap(), Some("engineer".to_string()));
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_insert_multiple() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("alice", "1".to_string()).unwrap();
        let dict = dict.inserting("bob", "2".to_string()).unwrap();
        let dict = dict.inserting("charlie", "3".to_string()).unwrap();
        assert_eq!(dict.len(), 3);
        assert_eq!(dict.get("alice").unwrap(), Some("1".to_string()));
        assert_eq!(dict.get("bob").unwrap(), Some("2".to_string()));
        assert_eq!(dict.get("charlie").unwrap(), Some("3".to_string()));
    }

    #[test]
    fn test_get_missing_key() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("alice", "1".to_string()).unwrap();
        assert_eq!(dict.get("bob").unwrap(), None);
    }

    #[test]
    fn test_insert_shared_prefix() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("apple", "1".to_string()).unwrap();
        let dict = dict.inserting("application", "2".to_string()).unwrap();
        assert_eq!(dict.get("apple").unwrap(), Some("1".to_string()));
        assert_eq!(dict.get("application").unwrap(), Some("2".to_string()));
        assert_eq!(dict.len(), 2);
    }

    #[test]
    fn test_mutating() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("key", "old".to_string()).unwrap();
        let dict = dict.mutating("key", "new".to_string()).unwrap();
        assert_eq!(dict.get("key").unwrap(), Some("new".to_string()));
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_delete() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("a", "1".to_string()).unwrap();
        let dict = dict.inserting("b", "2".to_string()).unwrap();
        let dict = dict.deleting("a").unwrap();
        assert_eq!(dict.get("a").unwrap(), None);
        assert_eq!(dict.get("b").unwrap(), Some("2".to_string()));
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_delete_nonexistent() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("a", "1".to_string()).unwrap();
        let dict = dict.deleting("b").unwrap();
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_all_keys() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("banana", "1".to_string()).unwrap();
        let dict = dict.inserting("apple", "2".to_string()).unwrap();
        let dict = dict.inserting("cherry", "3".to_string()).unwrap();
        let mut keys = dict.all_keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_sorted_keys() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("cherry", "3".to_string()).unwrap();
        let dict = dict.inserting("apple", "1".to_string()).unwrap();
        let dict = dict.inserting("banana", "2".to_string()).unwrap();
        let keys = dict.sorted_keys(10, None).unwrap();
        assert_eq!(keys, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_sorted_keys_with_pagination() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("a", "1".to_string()).unwrap();
        let dict = dict.inserting("b", "2".to_string()).unwrap();
        let dict = dict.inserting("c", "3".to_string()).unwrap();
        let dict = dict.inserting("d", "4".to_string()).unwrap();
        let keys = dict.sorted_keys(2, Some("a")).unwrap();
        assert_eq!(keys, vec!["b", "c"]);
    }

    #[test]
    fn test_all_keys_and_values() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("x", "10".to_string()).unwrap();
        let dict = dict.inserting("y", "20".to_string()).unwrap();
        let mut pairs = dict.all_keys_and_values().unwrap();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            pairs,
            vec![
                ("x".to_string(), "10".to_string()),
                ("y".to_string(), "20".to_string()),
            ]
        );
    }

    #[test]
    fn test_immutability() {
        let dict1 = MerkleDictionary::<String>::new();
        let dict2 = dict1.inserting("key", "value".to_string()).unwrap();
        assert!(dict1.is_empty());
        assert_eq!(dict2.len(), 1);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict.inserting("alice", "1".to_string()).unwrap();
        let dict = dict.inserting("bob", "2".to_string()).unwrap();
        let data = dict.to_data().unwrap();
        let restored = MerkleDictionary::<String>::from_data(&data).unwrap();
        assert_eq!(restored.len(), dict.len());
    }

    #[test]
    fn test_empty_key_rejected() {
        let dict = MerkleDictionary::<String>::new();
        assert!(dict.inserting("", "value".to_string()).is_err());
    }

    #[test]
    fn test_diff() {
        let old = MerkleDictionary::<String>::new();
        let old = old.inserting("a", "1".to_string()).unwrap();
        let old = old.inserting("b", "2".to_string()).unwrap();

        let new = old.inserting("c", "3".to_string()).unwrap();
        let new = new.deleting("a").unwrap();
        let new = new.mutating("b", "22".to_string()).unwrap();

        let diff = new.diff(&old).unwrap();
        assert!(diff.inserted.contains_key("c"));
        assert!(diff.deleted.contains_key("a"));
        assert!(diff.modified.contains_key("b"));
    }

    #[test]
    fn test_apply_transform() {
        let dict = MerkleDictionary::<String>::new();
        let dict = dict
            .apply_transform("key", &Transform::Insert("val".to_string()))
            .unwrap();
        assert_eq!(dict.get("key").unwrap(), Some("val".to_string()));

        let dict = dict
            .apply_transform("key", &Transform::Update("new".to_string()))
            .unwrap();
        assert_eq!(dict.get("key").unwrap(), Some("new".to_string()));

        let dict = dict.apply_transform("key", &Transform::Delete).unwrap();
        assert_eq!(dict.get("key").unwrap(), None);
    }
}
