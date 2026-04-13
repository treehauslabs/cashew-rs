/// A parsed query operation on a Merkle structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CashewExpression {
    /// Get value at key.
    Get(String),
    /// Array: get value at index.
    GetAt(usize),
    /// Get all keys.
    Keys,
    /// Get sorted keys with optional pagination.
    SortedKeys {
        limit: Option<usize>,
        after: Option<String>,
    },
    /// Get all values.
    Values,
    /// Get sorted values with optional pagination.
    SortedValues {
        limit: Option<usize>,
        after: Option<String>,
    },
    /// Get count of entries.
    Count,
    /// Check if a key exists.
    Contains(String),
    /// Array: get first element.
    First,
    /// Array: get last element.
    Last,
    /// Insert a new key-value pair.
    Insert { key: String, value: String },
    /// Update an existing key-value pair.
    Update { key: String, value: String },
    /// Insert or update.
    Set { key: String, value: String },
    /// Delete a key.
    Delete(String),
    /// Array: append a value.
    Append(String),
}
