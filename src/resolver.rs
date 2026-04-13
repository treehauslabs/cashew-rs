/// Strategy for resolving (hydrating) nodes in the Merkle tree.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolutionStrategy {
    /// Fetch only this one header (CID -> node), no recursion.
    Targeted,
    /// Transitively fetch entire subtree.
    Recursive,
    /// Load trie structure (keys enumerable), leave leaf headers unresolved.
    List,
    /// Load limited sorted range of keys.
    Range { after: Option<String>, limit: usize },
}
