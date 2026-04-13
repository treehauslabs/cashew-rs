use crate::error::Result;
use array_trie::ArrayTrie;

use crate::resolver::ResolutionStrategy;

/// Fetches serialized node data by CID.
#[async_trait::async_trait]
pub trait Fetcher: Send + Sync {
    async fn fetch(&self, raw_cid: &str) -> Result<Vec<u8>>;
}

/// Stores serialized node data by CID.
pub trait Storer: Send + Sync {
    fn store(&self, raw_cid: &str, data: &[u8]) -> Result<()>;
}

/// A fetcher that is also aware of Volume boundaries.
#[async_trait::async_trait]
pub trait VolumeAwareFetcher: Fetcher {
    async fn provide(&self, root_cid: &str, paths: &ArrayTrie<ResolutionStrategy>) -> Result<()>;
}

/// Combined fetcher + key provider for transparent decryption.
pub trait KeyProvidingFetcher: Fetcher + crate::encryption::KeyProvider {}
impl<T: Fetcher + crate::encryption::KeyProvider> KeyProvidingFetcher for T {}
