//! In-memory search result cache scoped to a single provider instance.
//!
//! N2 uses one `CachedSearchProvider` per `evaluate_stage_pipeline()` call.
//! Process-wide or Redis-backed caching is intentionally deferred.
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;

use super::{SearchProvider, normalize_cache_key};
use crate::stage_pipeline::factuality_types::SearchSnippet;
use crate::types::ValidatorError;

pub struct CachedSearchProvider<P: SearchProvider> {
    inner: P,
    cache: Mutex<HashMap<String, Vec<SearchSnippet>>>,
    hits: AtomicU32,
    misses: AtomicU32,
}

impl<P: SearchProvider> CachedSearchProvider<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            cache: Mutex::new(HashMap::new()),
            hits: AtomicU32::new(0),
            misses: AtomicU32::new(0),
        }
    }

    pub fn cache_len(&self) -> usize {
        self.cache.lock().map(|cache| cache.len()).unwrap_or(0)
    }

    pub fn cache_stats(&self) -> (u32, u32) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }
}

#[async_trait]
impl<P: SearchProvider + Sync> SearchProvider for CachedSearchProvider<P> {
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchSnippet>, ValidatorError> {
        let key = normalize_cache_key(query);

        if let Ok(cache) = self.cache.lock()
            && let Some(cached) = cache.get(&key)
        {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(cached.iter().take(limit).cloned().collect());
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        let snippets = self.inner.search(query, limit).await?;

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key, snippets.clone());
        }

        Ok(snippets.into_iter().take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::mock::MockSearchProvider;

    #[tokio::test]
    async fn cache_hits_on_second_lookup() {
        let provider =
            CachedSearchProvider::new(MockSearchProvider::with_snippets(vec![SearchSnippet {
                title: Some("Title".to_string()),
                snippet: "Snippet text".to_string(),
                url: Some("https://example.com".to_string()),
            }]));

        let first = provider
            .search("CSPR APY is 8%", 3)
            .await
            .expect("first search");
        assert_eq!(first.len(), 1);
        assert_eq!(provider.cache_len(), 1);
        assert_eq!(provider.cache_stats(), (0, 1));

        let second = provider
            .search("cspr   apy is 8%", 3)
            .await
            .expect("second search");
        assert_eq!(second, first);
        assert_eq!(provider.cache_stats(), (1, 1));
    }

    #[tokio::test]
    async fn different_claims_miss_cache() {
        let provider = CachedSearchProvider::new(MockSearchProvider::default());

        provider.search("claim one", 3).await.expect("first");
        provider.search("claim two", 3).await.expect("second");
        assert_eq!(provider.cache_len(), 2);
    }

    #[tokio::test]
    async fn separate_provider_instances_have_separate_caches() {
        let provider_a =
            CachedSearchProvider::new(MockSearchProvider::with_snippets(vec![SearchSnippet {
                title: Some("Title".to_string()),
                snippet: "Snippet text".to_string(),
                url: None,
            }]));
        let provider_b = CachedSearchProvider::new(MockSearchProvider::default());

        provider_a
            .search("same claim", 3)
            .await
            .expect("first search");
        assert_eq!(provider_a.cache_len(), 1);
        assert_eq!(provider_b.cache_len(), 0);
    }
}
