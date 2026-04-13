use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::Result;
use crate::merkle::dictionary::MerkleDictionary;

/// An append-only ordered collection built on top of `MerkleDictionary`.
///
/// Uses 256-bit binary string keys derived from integer indices to
/// preserve lexicographic ordering in the radix trie.
///
/// # Examples
/// ```
/// use cashew::MerkleArray;
///
/// let arr = MerkleArray::<String>::new();
/// let arr = arr.append("first".to_string()).unwrap();
/// let arr = arr.append("second".to_string()).unwrap();
///
/// assert_eq!(arr.get_at(0).unwrap(), Some("first".to_string()));
/// assert_eq!(arr.get_at(1).unwrap(), Some("second".to_string()));
/// assert_eq!(arr.len(), 2);
/// ```
#[derive(Clone, Debug)]
pub struct MerkleArray<V> {
    inner: MerkleDictionary<V>,
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> MerkleArray<V> {
    pub fn new() -> Self {
        Self {
            inner: MerkleDictionary::new(),
        }
    }

    pub fn from_dictionary(dict: MerkleDictionary<V>) -> Self {
        Self { inner: dict }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Converts an integer index to a 256-bit binary string key.
    pub fn binary_key(index: usize) -> String {
        format!("{:0>256b}", index)
    }

    /// Returns the value at the given index.
    pub fn get_at(&self, index: usize) -> Result<Option<V>> {
        let key = Self::binary_key(index);
        self.inner.get(&key)
    }

    /// Appends an element, returning a new array.
    pub fn append(&self, value: V) -> Result<Self>
    where
        V: fmt::Display,
    {
        let key = Self::binary_key(self.len());
        let new_inner = self.inner.inserting(&key, value)?;
        Ok(Self { inner: new_inner })
    }

    /// Returns the first element.
    pub fn first(&self) -> Result<Option<V>> {
        if self.is_empty() {
            return Ok(None);
        }
        self.get_at(0)
    }

    /// Returns the last element.
    pub fn last(&self) -> Result<Option<V>> {
        if self.is_empty() {
            return Ok(None);
        }
        self.get_at(self.len() - 1)
    }

    /// Appends all elements from another array.
    pub fn append_all(&self, other: &Self) -> Result<Self>
    where
        V: fmt::Display,
    {
        let mut result = self.clone();
        for i in 0..other.len() {
            if let Some(v) = other.get_at(i)? {
                result = result.append(v)?;
            }
        }
        Ok(result)
    }

    /// Returns a reference to the inner dictionary.
    pub fn as_dictionary(&self) -> &MerkleDictionary<V> {
        &self.inner
    }

    /// Serializes to JSON bytes.
    pub fn to_data(&self) -> Result<Vec<u8>> {
        self.inner.to_data()
    }

    /// Deserializes from JSON bytes.
    pub fn from_data(data: &[u8]) -> Result<Self> {
        Ok(Self {
            inner: MerkleDictionary::from_data(data)?,
        })
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> Default for MerkleArray<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + fmt::Display + Serialize + for<'de> Deserialize<'de>> fmt::Display
    for MerkleArray<V>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MerkleArray(count={})", self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_array_is_empty() {
        let arr = MerkleArray::<String>::new();
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn test_append_and_get() {
        let arr = MerkleArray::<String>::new();
        let arr = arr.append("first".to_string()).unwrap();
        let arr = arr.append("second".to_string()).unwrap();
        assert_eq!(arr.get_at(0).unwrap(), Some("first".to_string()));
        assert_eq!(arr.get_at(1).unwrap(), Some("second".to_string()));
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_first_and_last() {
        let arr = MerkleArray::<String>::new();
        assert_eq!(arr.first().unwrap(), None);
        assert_eq!(arr.last().unwrap(), None);

        let arr = arr.append("a".to_string()).unwrap();
        let arr = arr.append("b".to_string()).unwrap();
        let arr = arr.append("c".to_string()).unwrap();
        assert_eq!(arr.first().unwrap(), Some("a".to_string()));
        assert_eq!(arr.last().unwrap(), Some("c".to_string()));
    }

    #[test]
    fn test_append_all() {
        let arr1 = MerkleArray::<String>::new();
        let arr1 = arr1.append("a".to_string()).unwrap();
        let arr2 = MerkleArray::<String>::new();
        let arr2 = arr2.append("b".to_string()).unwrap();
        let arr2 = arr2.append("c".to_string()).unwrap();

        let merged = arr1.append_all(&arr2).unwrap();
        assert_eq!(merged.len(), 3);
        assert_eq!(merged.get_at(0).unwrap(), Some("a".to_string()));
        assert_eq!(merged.get_at(1).unwrap(), Some("b".to_string()));
        assert_eq!(merged.get_at(2).unwrap(), Some("c".to_string()));
    }

    #[test]
    fn test_binary_key_ordering() {
        let k0 = MerkleArray::<String>::binary_key(0);
        let k1 = MerkleArray::<String>::binary_key(1);
        let k100 = MerkleArray::<String>::binary_key(100);
        assert!(k0 < k1);
        assert!(k1 < k100);
        assert_eq!(k0.len(), 256);
    }

    #[test]
    fn test_immutability() {
        let arr1 = MerkleArray::<String>::new();
        let arr2 = arr1.append("x".to_string()).unwrap();
        assert!(arr1.is_empty());
        assert_eq!(arr2.len(), 1);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let arr = MerkleArray::<String>::new();
        let arr = arr.append("hello".to_string()).unwrap();
        let arr = arr.append("world".to_string()).unwrap();
        let data = arr.to_data().unwrap();
        let restored = MerkleArray::<String>::from_data(&data).unwrap();
        assert_eq!(restored.len(), 2);
    }
}
