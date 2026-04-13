use std::collections::BTreeMap;

/// Represents the differences between two versions of a Merkle structure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CashewDiff {
    /// Keys that were inserted: key -> value.
    pub inserted: BTreeMap<String, String>,
    /// Keys that were deleted: key -> old_value.
    pub deleted: BTreeMap<String, String>,
    /// Keys that were modified.
    pub modified: BTreeMap<String, ModifiedEntry>,
}

/// A single modified entry in a diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifiedEntry {
    /// The old CID or value.
    pub old: String,
    /// The new CID or value.
    pub new: String,
    /// Recursive diff if the values are nested structures.
    pub children: CashewDiff,
}

impl CashewDiff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.inserted.is_empty() && self.deleted.is_empty() && self.modified.is_empty()
    }

    pub fn change_count(&self) -> usize {
        self.inserted.len() + self.deleted.len() + self.modified.len()
    }
}
