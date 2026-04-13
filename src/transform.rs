/// A mutation operation to apply to a Merkle dictionary key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transform {
    /// Insert a new value (fails if key exists).
    Insert(String),
    /// Update an existing value (fails if key doesn't exist).
    Update(String),
    /// Delete the value at this key.
    Delete,
}
