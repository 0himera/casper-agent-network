use std::collections::HashMap;

use async_trait::async_trait;

use super::SearchProvider;
use crate::stage_pipeline::factuality_types::SearchSnippet;
use crate::types::ValidatorError;

#[derive(Debug, Clone, Default)]
pub struct MockSearchProvider {
    responses: HashMap<String, Vec<SearchSnippet>>,
    default: Vec<SearchSnippet>,
}

impl MockSearchProvider {
    pub fn with_snippets(snippets: Vec<SearchSnippet>) -> Self {
        Self {
            responses: HashMap::new(),
            default: snippets,
        }
    }

    pub fn with_query_responses(responses: HashMap<String, Vec<SearchSnippet>>) -> Self {
        Self {
            responses,
            default: Vec::new(),
        }
    }

    pub fn with_error() -> MockSearchProviderError {
        MockSearchProviderError
    }
}

#[derive(Debug, Clone, Default)]
pub struct MockSearchProviderError;

#[async_trait]
impl SearchProvider for MockSearchProvider {
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchSnippet>, ValidatorError> {
        let key = super::normalize_cache_key(query);
        let snippets = self
            .responses
            .get(&key)
            .cloned()
            .unwrap_or_else(|| self.default.clone());
        Ok(snippets.into_iter().take(limit).collect())
    }
}

#[async_trait]
impl SearchProvider for MockSearchProviderError {
    async fn search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<SearchSnippet>, ValidatorError> {
        Err(ValidatorError::Search("mock search failure".to_string()))
    }
}

pub fn provider_for_mock_mode(mode: Option<&str>) -> Box<dyn SearchProvider> {
    match mode {
        Some("error") => Box::new(MockSearchProviderError),
        Some("contradicted") => Box::new(MockSearchProvider::with_snippets(vec![SearchSnippet {
            title: Some("Contradiction".to_string()),
            snippet: "Evidence contradicts the claim.".to_string(),
            url: Some("https://example.com/contra".to_string()),
        }])),
        Some("unverifiable") => Box::new(MockSearchProvider::default()),
        _ => Box::new(MockSearchProvider::with_snippets(vec![SearchSnippet {
            title: Some("Support".to_string()),
            snippet: "Evidence supports the claim.".to_string(),
            url: Some("https://example.com/support".to_string()),
        }])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_provider_respects_limit() {
        let provider = MockSearchProvider::with_snippets(vec![
            SearchSnippet {
                title: Some("A".to_string()),
                snippet: "one".to_string(),
                url: None,
            },
            SearchSnippet {
                title: Some("B".to_string()),
                snippet: "two".to_string(),
                url: None,
            },
        ]);

        let results = provider.search("query", 1).await.expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].snippet, "one");
    }

    #[tokio::test]
    async fn mock_provider_error_returns_search_error() {
        let provider = MockSearchProviderError;
        let err = provider
            .search("query", 3)
            .await
            .expect_err("expected error");
        assert!(matches!(err, ValidatorError::Search(_)));
    }
}
