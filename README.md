# cashew

Content-addressed Merkle data structures with encryption, proofs, and a query language.

## Features

- **MerkleDictionary** — persistent key-value store backed by a content-addressed compressed radix trie. All mutations return new instances; the original remains unchanged.
- **MerkleArray** — append-only ordered collection using 256-bit binary string keys for lexicographic ordering in the radix trie.
- **MerkleSet** — string set backed by MerkleDictionary, with union, intersection, difference, and symmetric difference operations.
- **CID computation** — SHA-256 hashing with CID v1 (dag-json codec) via the IPLD ecosystem crates.
- **AES-256-GCM encryption** — per-node encryption with EncryptionInfo metadata, supporting targeted, list, and recursive strategies.
- **Deterministic serialization** — BTreeMap-based sorted entries ensure identical JSON output for identical data, making CIDs stable across platforms.
- **Structural diffing** — `CashewDiff` computes inserted, deleted, and modified keys between two dictionary versions.
- **Query language** — parse pipe-separated commands (`get "users" | keys sorted limit 10`) into expression trees for evaluation.
- **Sparse Merkle proofs** — proof types for insertion, mutation, deletion, and existence.
- **Lazy resolution** — headers store CIDs and defer node loading via the async `Fetcher` trait.
- **Batch transforms** — apply `Insert`, `Update`, and `Delete` transforms via `ArrayTrie<Transform>`.

## Usage

```rust
use cashew::{MerkleDictionary, MerkleSet, MerkleArray, CashewParser};

// Dictionary
let dict = MerkleDictionary::<String>::new();
let dict = dict.inserting("alice", "engineer".to_string()).unwrap();
let dict = dict.inserting("bob", "designer".to_string()).unwrap();
assert_eq!(dict.get("alice").unwrap(), Some("engineer".to_string()));

// Set
let set = MerkleSet::new();
let set = set.insert("alice").unwrap();
let set = set.insert("bob").unwrap();
assert!(set.contains("alice").unwrap());

// Array
let arr = MerkleArray::<String>::new();
let arr = arr.append("first".to_string()).unwrap();
assert_eq!(arr.get_at(0).unwrap(), Some("first".to_string()));

// Query parser
let exprs = CashewParser::parse(r#"get "users" | keys sorted limit 10"#).unwrap();
```

## Dependencies

- [trie-dictionary](https://github.com/treehauslabs/trie-dictionary) — compressed trie with path compression
- [array-trie](https://github.com/treehauslabs/array-trie) — trie keyed by `Vec<String>` paths
- [cid](https://crates.io/crates/cid), [multihash](https://crates.io/crates/multihash) — IPLD content identifiers
- [aes-gcm](https://crates.io/crates/aes-gcm) — AES-256-GCM encryption
- [serde](https://crates.io/crates/serde) / [serde_json](https://crates.io/crates/serde_json) — serialization

## License

MIT
