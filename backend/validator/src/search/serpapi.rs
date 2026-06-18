use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use super::SearchProvider;
use crate::stage_pipeline::factuality_types::SearchSnippet;
use crate::types::ValidatorError;

const SERPAPI_ENDPOINT: &str = "https://serpapi.com/search";

#[derive(Debug, Clone)]
pub struct SerpApiProvider {
    api_key: String,
    client: Client,
}

impl SerpApiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    pub fn from_optional_key(api_key: Option<String>) -> Result<Self, ValidatorError> {
        let api_key = api_key
            .filter(|key| !key.is_empty())
            .ok_or_else(|| ValidatorError::Search("SERPAPI_API_KEY is missing".to_string()))?;
        Ok(Self::new(api_key))
    }

    pub fn parse_organic_results(json: &Value, limit: usize) -> Vec<SearchSnippet> {
        json.get("organic_results")
            .and_then(Value::as_array)
            .map(|results| {
                results
                    .iter()
                    .filter_map(parse_organic_result)
                    .take(limit)
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn parse_organic_result(value: &Value) -> Option<SearchSnippet> {
    let snippet = value
        .get("snippet")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())?;

    Some(SearchSnippet {
        title: value
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string),
        snippet: snippet.to_string(),
        url: value
            .get("link")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

#[async_trait]
impl SearchProvider for SerpApiProvider {
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchSnippet>, ValidatorError> {
        let response = self
            .client
            .get(SERPAPI_ENDPOINT)
            .query(&[
                ("engine", "google"),
                ("q", query),
                ("api_key", self.api_key.as_str()),
            ])
            .send()
            .await
            .map_err(|err| ValidatorError::Search(format!("SerpAPI request failed: {err}")))?;

        if !response.status().is_success() {
            return Err(ValidatorError::Search(format!(
                "SerpAPI returned HTTP {}",
                response.status()
            )));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|err| ValidatorError::Search(format!("SerpAPI JSON parse failed: {err}")))?;

        Ok(Self::parse_organic_results(&json, limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_organic_results_extracts_top_snippets() {
        let payload = json!({
            "organic_results": [
                {
                    "title": "Result A",
                    "link": "https://example.com/a",
                    "snippet": "First snippet"
                },
                {
                    "title": "Result B",
                    "link": "https://example.com/b",
                    "snippet": "Second snippet"
                }
            ]
        });

        let snippets = SerpApiProvider::parse_organic_results(&payload, 3);
        assert_eq!(snippets.len(), 2);
        assert_eq!(snippets[0].title.as_deref(), Some("Result A"));
        assert_eq!(snippets[0].snippet, "First snippet");
        assert_eq!(snippets[0].url.as_deref(), Some("https://example.com/a"));
    }

    #[test]
    fn parse_organic_results_respects_limit() {
        let payload = json!({
            "organic_results": [
                {"snippet": "one"},
                {"snippet": "two"},
                {"snippet": "three"}
            ]
        });

        let snippets = SerpApiProvider::parse_organic_results(&payload, 2);
        assert_eq!(snippets.len(), 2);
    }

    #[test]
    fn parse_organic_results_skips_empty_snippets() {
        let payload = json!({
            "organic_results": [
                {"snippet": ""},
                {"title": "No snippet field"}
            ]
        });

        assert!(SerpApiProvider::parse_organic_results(&payload, 3).is_empty());
    }
}
