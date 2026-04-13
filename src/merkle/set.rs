use crate::error::Result;
use crate::merkle::dictionary::MerkleDictionary;

/// A set with string members, backed by a `MerkleDictionary<String>`.
///
/// Stores members as keys with empty string values.
///
/// # Examples
/// ```
/// use cashew::MerkleSet;
///
/// let set = MerkleSet::new();
/// let set = set.insert("alice").unwrap();
/// let set = set.insert("bob").unwrap();
///
/// assert!(set.contains("alice").unwrap());
/// assert!(!set.contains("charlie").unwrap());
/// assert_eq!(set.len(), 2);
/// ```
#[derive(Clone, Debug)]
pub struct MerkleSet {
    inner: MerkleDictionary<String>,
}

impl MerkleSet {
    pub fn new() -> Self {
        Self {
            inner: MerkleDictionary::new(),
        }
    }

    pub fn from_dictionary(dict: MerkleDictionary<String>) -> Self {
        Self { inner: dict }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Inserts a member.
    pub fn insert(&self, member: &str) -> Result<Self> {
        let new_inner = self.inner.inserting(member, String::new())?;
        Ok(Self { inner: new_inner })
    }

    /// Removes a member.
    pub fn remove(&self, member: &str) -> Result<Self> {
        let new_inner = self.inner.deleting(member)?;
        Ok(Self { inner: new_inner })
    }

    /// Returns true if the set contains the member.
    pub fn contains(&self, member: &str) -> Result<bool> {
        Ok(self.inner.get(member)?.is_some())
    }

    /// Returns all members.
    pub fn members(&self) -> Result<Vec<String>> {
        self.inner.all_keys()
    }

    /// Returns sorted members.
    pub fn sorted_members(&self) -> Result<Vec<String>> {
        self.inner.sorted_keys(usize::MAX, None)
    }

    /// Returns the union of two sets.
    pub fn union(&self, other: &Self) -> Result<Self> {
        let mut result = self.clone();
        for member in other.members()? {
            if !result.contains(&member)? {
                result = result.insert(&member)?;
            }
        }
        Ok(result)
    }

    /// Returns the intersection of two sets.
    pub fn intersection(&self, other: &Self) -> Result<Self> {
        let mut result = Self::new();
        for member in self.members()? {
            if other.contains(&member)? {
                result = result.insert(&member)?;
            }
        }
        Ok(result)
    }

    /// Returns members in self but not in other.
    pub fn subtracting(&self, other: &Self) -> Result<Self> {
        let mut result = self.clone();
        for member in other.members()? {
            if result.contains(&member)? {
                result = result.remove(&member)?;
            }
        }
        Ok(result)
    }

    /// Returns members in exactly one of the two sets.
    pub fn symmetric_difference(&self, other: &Self) -> Result<Self> {
        let a_minus_b = self.subtracting(other)?;
        let b_minus_a = other.subtracting(self)?;
        a_minus_b.union(&b_minus_a)
    }

    pub fn as_dictionary(&self) -> &MerkleDictionary<String> {
        &self.inner
    }

    pub fn to_data(&self) -> Result<Vec<u8>> {
        self.inner.to_data()
    }

    pub fn from_data(data: &[u8]) -> Result<Self> {
        Ok(Self {
            inner: MerkleDictionary::from_data(data)?,
        })
    }
}

impl Default for MerkleSet {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MerkleSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MerkleSet(count={})", self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_set_is_empty() {
        let set = MerkleSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_insert_and_contains() {
        let set = MerkleSet::new();
        let set = set.insert("alice").unwrap();
        let set = set.insert("bob").unwrap();
        assert!(set.contains("alice").unwrap());
        assert!(set.contains("bob").unwrap());
        assert!(!set.contains("charlie").unwrap());
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_remove() {
        let set = MerkleSet::new();
        let set = set.insert("a").unwrap();
        let set = set.insert("b").unwrap();
        let set = set.remove("a").unwrap();
        assert!(!set.contains("a").unwrap());
        assert!(set.contains("b").unwrap());
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_members() {
        let set = MerkleSet::new();
        let set = set.insert("cherry").unwrap();
        let set = set.insert("apple").unwrap();
        let set = set.insert("banana").unwrap();
        let mut members = set.members().unwrap();
        members.sort();
        assert_eq!(members, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_union() {
        let a = MerkleSet::new();
        let a = a.insert("x").unwrap();
        let a = a.insert("y").unwrap();
        let b = MerkleSet::new();
        let b = b.insert("y").unwrap();
        let b = b.insert("z").unwrap();
        let u = a.union(&b).unwrap();
        assert_eq!(u.len(), 3);
        assert!(u.contains("x").unwrap());
        assert!(u.contains("y").unwrap());
        assert!(u.contains("z").unwrap());
    }

    #[test]
    fn test_intersection() {
        let a = MerkleSet::new();
        let a = a.insert("x").unwrap();
        let a = a.insert("y").unwrap();
        let b = MerkleSet::new();
        let b = b.insert("y").unwrap();
        let b = b.insert("z").unwrap();
        let i = a.intersection(&b).unwrap();
        assert_eq!(i.len(), 1);
        assert!(i.contains("y").unwrap());
    }

    #[test]
    fn test_subtracting() {
        let a = MerkleSet::new();
        let a = a.insert("x").unwrap();
        let a = a.insert("y").unwrap();
        let b = MerkleSet::new();
        let b = b.insert("y").unwrap();
        let diff = a.subtracting(&b).unwrap();
        assert_eq!(diff.len(), 1);
        assert!(diff.contains("x").unwrap());
    }

    #[test]
    fn test_symmetric_difference() {
        let a = MerkleSet::new();
        let a = a.insert("x").unwrap();
        let a = a.insert("y").unwrap();
        let b = MerkleSet::new();
        let b = b.insert("y").unwrap();
        let b = b.insert("z").unwrap();
        let sd = a.symmetric_difference(&b).unwrap();
        assert_eq!(sd.len(), 2);
        assert!(sd.contains("x").unwrap());
        assert!(sd.contains("z").unwrap());
        assert!(!sd.contains("y").unwrap());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let set = MerkleSet::new();
        let set = set.insert("alice").unwrap();
        let set = set.insert("bob").unwrap();
        let data = set.to_data().unwrap();
        let restored = MerkleSet::from_data(&data).unwrap();
        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn test_immutability() {
        let s1 = MerkleSet::new();
        let s2 = s1.insert("x").unwrap();
        assert!(s1.is_empty());
        assert_eq!(s2.len(), 1);
    }
}
