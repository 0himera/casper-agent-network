use async_trait::async_trait;

use crate::stage_pipeline::factuality_types::SearchSnippet;
use crate::types::ValidatorError;

#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &str, limit: usize)
    -> Result<Vec<SearchSnippet>, ValidatorError>;
}

#[async_trait]
impl SearchProvider for Box<dyn SearchProvider> {
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchSnippet>, ValidatorError> {
        self.as_ref().search(query, limit).await
    }
}

pub fn normalize_cache_key(query: &str) -> String {
    query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

pub mod cache;
pub mod mock;
pub mod serpapi;

pub use cache::CachedSearchProvider;
pub use mock::MockSearchProvider;
pub use serpapi::SerpApiProvider;
