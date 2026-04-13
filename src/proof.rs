use serde::{Deserialize, Serialize};

/// Type of sparse Merkle proof to generate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SparseMerkleProof {
    /// Prove that a key does NOT exist (for safe insertion).
    Insertion = 1,
    /// Prove that a key exists and can be mutated.
    Mutation = 2,
    /// Prove that a key exists and can be deleted.
    Deletion = 3,
    /// Prove that a key exists.
    Existence = 4,
}
