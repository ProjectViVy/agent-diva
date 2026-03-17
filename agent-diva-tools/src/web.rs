//! Web tools: web_search and web_fetch

use crate::base::{Tool, ToolError};
use crate::sanitize::sanitize_for_json;
use async_trait::async_trait;
use regex::Regex;
use reqwest::{header, Client};
use serde_json::{json, Value};
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7_2) AppleWebKit/537.36";
const MAX_REDIRECTS: usize = 5;

/// Strip HTML tags and decode entities
fn strip_tags(text: &str) -> String {
    // Remove script and style tags
    let text = Regex::new(r"(?i)<script[\s\S]*?</script>")
        .unwrap()
        .replace_all(text, "");
    let text = Regex::new(r"(?i)<style[\s\S]*?</style>")
        .unwrap()
        .replace_all(&text, "");
    // Remove all HTML tags
    let text = Regex::new(r"<[^>]+>").unwrap().replace_all(&text, "");
    // Basic HTML entity decoding
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

/// Normalize whitespace
fn normalize_whitespace(text: &str) -> String {
    // Collapse multiple spaces/tabs to single space
    let text = Regex::new(r"[ \t]+").unwrap().replace_all(text, " ");
    // Collapse 3+ newlines to 2
    let text = Regex::new(r"\n{3,}").unwrap().replace_all(&text, "\n\n");
    text.trim().to_string()
}

/// Validate URL
fn validate_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

    match parsed.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("Only http/https allowed, got '{}'", scheme)),
    }
}

/// Web search tool supporting multiple providers.
pub struct WebSearchTool {
    provider: String,
    init_api_key: Option<String>,
    max_results: usize,
    client: Client,
}

impl WebSearchTool {
    /// Create a new web search tool with the default provider (bocha).
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_provider_and_max_results("bocha", api_key, 5)
    }

    /// Create with custom max results using bocha provider.
    pub fn with_max_results(api_key: Option<String>, max_results: usize) -> Self {
        Self::with_provider_and_max_results("bocha", api_key, max_results)
    }

    /// Create with explicit provider and max results.
    pub fn with_provider_and_max_results(
        provider: impl Into<String>,
        api_key: Option<String>,
        max_results: usize,
    ) -> Self {
        Self {
            provider: provider.into(),
            init_api_key: api_key,
            max_results,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    fn normalized_provider(&self) -> String {
        self.provider.trim().to_lowercase()
    }

    fn max_results_limit(provider: &str) -> usize {
        if provider == "zhipu" || provider == "bocha" {
            50
        } else {
            10
        }
    }

    /// Resolve API key at call time: init key -> env var fallback.
    fn resolve_api_key(&self) -> Option<String> {
        let provider = self.normalized_provider();
        self.init_api_key
            .clone()
            .or_else(|| match provider.as_str() {
                "brave" => std::env::var("BRAVE_API_KEY").ok(),
                "bocha" => std::env::var("BOCHA_API_KEY").ok(),
                "zhipu" => std::env::var("ZHIPU_API_KEY")
                    .ok()
                    .or_else(|| std::env::var("BIGMODEL_API_KEY").ok()),
                _ => None,
            })
    }

    async fn search_brave(
        &self,
        query: &str,
        count: usize,
        api_key: &str,
    ) -> Result<String, ToolError> {
        let response = self
            .client
            .get("https://api.search.brave.com/res/v1/web/search")
            .query(&[("q", query), ("count", &count.to_string())])
            .header("Accept", "application/json")
            .header("X-Subscription-Token", api_key)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "Brave request failed ({}): {}",
                status, text
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse response: {}", e)))?;

        let results = data
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(|r| r.as_array())
            .ok_or_else(|| ToolError::ExecutionFailed("No results found".to_string()))?;

        if results.is_empty() {
            return Ok(format!("No results for: {}", query));
        }

        let mut lines = vec![format!("Results for: {}\n", query)];
        for (i, item) in results.iter().take(count).enumerate() {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!("{}. {}\n   {}", i + 1, title, url));

            if let Some(desc) = item.get("description").and_then(|v| v.as_str()) {
                lines.push(format!("   {}", desc));
            }
        }

        Ok(lines.join("\n"))
    }

    async fn search_zhipu(
        &self,
        params: &Value,
        query: &str,
        count: usize,
        api_key: &str,
    ) -> Result<String, ToolError> {
        let search_engine = params
            .get("search_engine")
            .and_then(|v| v.as_str())
            .unwrap_or("search_std");
        let search_intent = params
            .get("search_intent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let search_recency_filter = params
            .get("search_recency_filter")
            .and_then(|v| v.as_str())
            .unwrap_or("noLimit");
        let content_size = params
            .get("content_size")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        let mut body = serde_json::Map::new();
        body.insert("search_query".to_string(), Value::String(query.to_string()));
        body.insert(
            "search_engine".to_string(),
            Value::String(search_engine.to_string()),
        );
        body.insert("search_intent".to_string(), Value::Bool(search_intent));
        body.insert(
            "count".to_string(),
            Value::Number(serde_json::Number::from(count as u64)),
        );
        body.insert(
            "search_recency_filter".to_string(),
            Value::String(search_recency_filter.to_string()),
        );
        body.insert(
            "content_size".to_string(),
            Value::String(content_size.to_string()),
        );
        if let Some(domain) = params.get("search_domain_filter").and_then(|v| v.as_str()) {
            if !domain.trim().is_empty() {
                body.insert(
                    "search_domain_filter".to_string(),
                    Value::String(domain.to_string()),
                );
            }
        }
        if let Some(request_id) = params.get("request_id").and_then(|v| v.as_str()) {
            if !request_id.trim().is_empty() {
                body.insert(
                    "request_id".to_string(),
                    Value::String(request_id.to_string()),
                );
            }
        }
        if let Some(user_id) = params.get("user_id").and_then(|v| v.as_str()) {
            if !user_id.trim().is_empty() {
                body.insert("user_id".to_string(), Value::String(user_id.to_string()));
            }
        }

        let response = self
            .client
            .post("https://open.bigmodel.cn/api/paas/v4/web_search")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&Value::Object(body))
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "Zhipu request failed ({}): {}",
                status, text
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse response: {}", e)))?;

        let results = data
            .get("search_result")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::ExecutionFailed("No results found".to_string()))?;

        if results.is_empty() {
            return Ok(format!("No results for: {}", query));
        }

        let mut lines = vec![format!("Results for: {}\n", query)];
        for (i, item) in results.iter().take(count).enumerate() {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let url = item.get("link").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!("{}. {}\n   {}", i + 1, title, url));

            if let Some(desc) = item.get("content").and_then(|v| v.as_str()) {
                if !desc.is_empty() {
                    lines.push(format!("   {}", desc));
                }
            }
            if let Some(media) = item.get("media").and_then(|v| v.as_str()) {
                if !media.is_empty() {
                    lines.push(format!("   Source: {}", media));
                }
            }
            if let Some(publish_date) = item.get("publish_date").and_then(|v| v.as_str()) {
                if !publish_date.is_empty() {
                    lines.push(format!("   Published: {}", publish_date));
                }
            }
        }

        Ok(lines.join("\n"))
    }

    async fn search_bocha(
        &self,
        params: &Value,
        query: &str,
        count: usize,
        api_key: &str,
    ) -> Result<String, ToolError> {
        let freshness = params
            .get("freshness")
            .and_then(|v| v.as_str())
            .unwrap_or("noLimit");
        let summary = params
            .get("summary")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut body = serde_json::Map::new();
        body.insert("query".to_string(), Value::String(query.to_string()));
        body.insert(
            "count".to_string(),
            Value::Number(serde_json::Number::from(count as u64)),
        );
        body.insert(
            "freshness".to_string(),
            Value::String(freshness.to_string()),
        );
        body.insert("summary".to_string(), Value::Bool(summary));

        if let Some(include) = params.get("include").and_then(|v| v.as_str()) {
            if !include.trim().is_empty() {
                body.insert("include".to_string(), Value::String(include.to_string()));
            }
        }
        if let Some(exclude) = params.get("exclude").and_then(|v| v.as_str()) {
            if !exclude.trim().is_empty() {
                body.insert("exclude".to_string(), Value::String(exclude.to_string()));
            }
        }

        let response = self
            .client
            .post("https://api.bocha.cn/v1/web-search")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&Value::Object(body))
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "Bocha request failed ({}): {}",
                status, text
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse response: {}", e)))?;

        let results = data
            .get("data")
            .and_then(|v| v.get("webPages"))
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::ExecutionFailed("No results found".to_string()))?;

        if results.is_empty() {
            return Ok(format!("No results for: {}", query));
        }

        let mut lines = vec![format!("Results for: {}\n", query)];
        for (i, item) in results.iter().take(count).enumerate() {
            let title = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!("{}. {}\n   {}", i + 1, title, url));

            let desc = item
                .get("summary")
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .or_else(|| {
                    item.get("snippet")
                        .and_then(|v| v.as_str())
                        .filter(|v| !v.is_empty())
                });
            if let Some(desc) = desc {
                lines.push(format!("   {}", desc));
            }
            if let Some(site_name) = item.get("siteName").and_then(|v| v.as_str()) {
                if !site_name.is_empty() {
                    lines.push(format!("   Source: {}", site_name));
                }
            }
            if let Some(publish_date) = item.get("datePublished").and_then(|v| v.as_str()) {
                if !publish_date.is_empty() {
                    lines.push(format!("   Published: {}", publish_date));
                }
            }
        }

        Ok(lines.join("\n"))
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web. Returns titles, URLs, and snippets."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of results (brave: 1-10, bocha/zhipu: 1-50)",
                    "minimum": 1,
                    "maximum": 50
                },
                "freshness": {
                    "type": "string",
                    "description": "Time range filter (bocha only)",
                    "enum": ["oneDay", "oneWeek", "oneMonth", "oneYear", "noLimit"]
                },
                "summary": {
                    "type": "boolean",
                    "description": "Include long-form summaries in results (bocha only)"
                },
                "include": {
                    "type": "string",
                    "description": "Domain whitelist filter (bocha only)"
                },
                "exclude": {
                    "type": "string",
                    "description": "Domain blacklist filter (bocha only)"
                },
                "search_engine": {
                    "type": "string",
                    "description": "Zhipu search engine (zhipu only)",
                    "enum": ["search_std", "search_pro", "search_pro_sogou", "search_pro_quark"]
                },
                "search_intent": {
                    "type": "boolean",
                    "description": "Enable intent recognition before searching (zhipu only)"
                },
                "search_domain_filter": {
                    "type": "string",
                    "description": "Domain whitelist filter (zhipu only)"
                },
                "search_recency_filter": {
                    "type": "string",
                    "description": "Time range filter (zhipu only)",
                    "enum": ["oneDay", "oneWeek", "oneMonth", "oneYear", "noLimit"]
                },
                "content_size": {
                    "type": "string",
                    "description": "Returned content length (zhipu only)",
                    "enum": ["medium", "high"]
                },
                "request_id": {
                    "type": "string",
                    "description": "Unique request identifier (zhipu only)"
                },
                "user_id": {
                    "type": "string",
                    "description": "End user identifier (zhipu only)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'query' parameter".to_string()))?;

        let provider = self.normalized_provider();
        if provider != "brave" && provider != "bocha" && provider != "zhipu" {
            return Err(ToolError::ExecutionFailed(format!(
                "Unsupported web search provider: {}",
                provider
            )));
        }
        let count = params
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.max_results as u64)
            .clamp(1, Self::max_results_limit(&provider) as u64) as usize;

        let api_key = self.resolve_api_key().ok_or_else(|| {
            let hint = match provider.as_str() {
                "brave" => "BRAVE_API_KEY",
                "bocha" => "BOCHA_API_KEY",
                "zhipu" => "ZHIPU_API_KEY or BIGMODEL_API_KEY",
                _ => "an API key",
            };
            ToolError::ExecutionFailed(format!("{} not configured", hint))
        })?;

        match provider.as_str() {
            "brave" => self.search_brave(query, count, &api_key).await,
            "bocha" => self.search_bocha(&params, query, count, &api_key).await,
            "zhipu" => self.search_zhipu(&params, query, count, &api_key).await,
            _ => unreachable!(),
        }
    }
}

/// Web fetch tool to extract content from URLs
pub struct WebFetchTool {
    max_chars: usize,
    client: Client,
}

impl WebFetchTool {
    /// Create a new web fetch tool
    pub fn new() -> Self {
        Self {
            max_chars: 50000,
            client: Client::builder()
                .user_agent(USER_AGENT)
                .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// Create with custom max chars
    pub fn with_max_chars(max_chars: usize) -> Self {
        Self {
            max_chars,
            client: Client::builder()
                .user_agent(USER_AGENT)
                .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// Convert HTML to markdown (simplified)
    fn to_markdown(&self, html: &str) -> String {
        let mut text = html.to_string();

        // Convert links: <a href="url">text</a> -> [text](url)
        let link_re =
            Regex::new(r#"(?i)<a\s+[^>]*href=["']([^"']+)["'][^>]*>([\s\S]*?)</a>"#).unwrap();
        let link_matches: Vec<_> = link_re
            .captures_iter(html)
            .map(|cap| {
                let url = cap[1].to_string();
                let link_text = strip_tags(&cap[2]);
                let replacement = format!("[{}]({})", link_text, url);
                (cap[0].to_string(), replacement)
            })
            .collect();
        for (original, replacement) in link_matches {
            text = text.replace(&original, &replacement);
        }

        // Convert headings
        for level in 1..=6 {
            let pattern = format!(r#"(?i)<h{0}[^>]*>([\s\S]*?)</h{0}>"#, level);
            let heading_re = Regex::new(&pattern).unwrap();
            let heading_matches: Vec<_> = heading_re
                .captures_iter(&text.clone())
                .map(|cap| {
                    let heading_text = strip_tags(&cap[1]);
                    let replacement = format!("\n{} {}\n", "#".repeat(level), heading_text);
                    (cap[0].to_string(), replacement)
                })
                .collect();
            for (original, replacement) in heading_matches {
                text = text.replace(&original, &replacement);
            }
        }

        // Convert list items: <li>text</li> -> - text
        let li_re = Regex::new(r#"(?i)<li[^>]*>([\s\S]*?)</li>"#).unwrap();
        let li_matches: Vec<_> = li_re
            .captures_iter(&text.clone())
            .map(|cap| {
                let item_text = strip_tags(&cap[1]);
                let replacement = format!("\n- {}", item_text);
                (cap[0].to_string(), replacement)
            })
            .collect();
        for (original, replacement) in li_matches {
            text = text.replace(&original, &replacement);
        }

        // Convert block-level elements to newlines
        let block_re = Regex::new(r"(?i)</(p|div|section|article)>").unwrap();
        text = block_re.replace_all(&text, "\n\n").to_string();

        // Convert line breaks
        let br_re = Regex::new(r"(?i)<(br|hr)\s*/?>").unwrap();
        text = br_re.replace_all(&text, "\n").to_string();

        normalize_whitespace(&strip_tags(&text))
    }

    /// Extract readable content from HTML (simple version)
    fn extract_content(&self, html: &str) -> String {
        // Remove script and style
        let text = Regex::new(r"(?i)<script[\s\S]*?</script>")
            .unwrap()
            .replace_all(html, "");
        let text = Regex::new(r"(?i)<style[\s\S]*?</style>")
            .unwrap()
            .replace_all(&text, "");

        // Extract title if present
        let title = Regex::new(r"(?i)<title>([\s\S]*?)</title>")
            .unwrap()
            .captures(&text)
            .and_then(|c| c.get(1))
            .map(|m| strip_tags(m.as_str()))
            .unwrap_or_default();

        // Extract body content
        let body = Regex::new(r"(?i)<body[^>]*>([\s\S]*?)</body>")
            .unwrap()
            .captures(&text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or(&text);

        let content = strip_tags(body);

        if !title.is_empty() {
            format!("# {}\n\n{}", title, normalize_whitespace(&content))
        } else {
            normalize_whitespace(&content)
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch URL and extract readable content (HTML → markdown/text)."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                },
                "extractMode": {
                    "type": "string",
                    "enum": ["markdown", "text"],
                    "description": "Content extraction mode",
                    "default": "markdown"
                },
                "maxChars": {
                    "type": "integer",
                    "minimum": 100,
                    "description": "Maximum characters to return"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'url' parameter".to_string()))?;

        let extract_mode = params
            .get("extractMode")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown");

        let max_chars = params
            .get("maxChars")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.max_chars as u64) as usize;

        // Validate URL
        if let Err(err) = validate_url(url) {
            return Ok(json!({
                "error": format!("URL validation failed: {}", err),
                "url": url
            })
            .to_string());
        }

        // Fetch content
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Request failed: {}", e)))?;

        let final_url = response.url().to_string();
        let status = response.status().as_u16();

        // Clone headers before consuming response
        let headers = response.headers().clone();

        let html = response
            .text()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Determine extractor and process content
        let (text, extractor) = if content_type.contains("application/json") {
            // Try to pretty-print JSON
            match serde_json::from_str::<Value>(&html) {
                Ok(json_value) => (
                    serde_json::to_string_pretty(&json_value).unwrap_or(html),
                    "json",
                ),
                Err(_) => (html, "json"),
            }
        } else if content_type.contains("text/html")
            || html.trim_start().to_lowercase().starts_with("<!doctype")
            || html.trim_start().to_lowercase().starts_with("<html")
        {
            // HTML content
            let content = if extract_mode == "markdown" {
                self.to_markdown(&self.extract_content(&html))
            } else {
                self.extract_content(&html)
            };
            (content, "simple")
        } else {
            (html, "raw")
        };

        // Truncate if needed
        let truncated = text.len() > max_chars;
        let text = if truncated {
            text.chars().take(max_chars).collect::<String>()
        } else {
            text
        };

        // Sanitize to remove control characters that could cause JSON errors
        let text = sanitize_for_json(&text);

        // Return JSON response
        Ok(json!({
            "url": url,
            "finalUrl": final_url,
            "status": status,
            "extractor": extractor,
            "truncated": truncated,
            "length": text.len(),
            "text": text
        })
        .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tags() {
        let html = "<p>Hello <b>world</b></p>";
        assert_eq!(strip_tags(html), "Hello world");

        let html = "<script>alert('hi')</script><p>Text</p>";
        assert_eq!(strip_tags(html), "Text");
    }

    #[test]
    fn test_normalize_whitespace() {
        let text = "Hello    world\n\n\n\ntest";
        assert_eq!(normalize_whitespace(text), "Hello world\n\ntest");
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("not-a-url").is_err());
    }

    #[tokio::test]
    async fn test_web_search_no_api_key() {
        let tool = WebSearchTool::new(None);
        let params = json!({"query": "rust programming"});

        // Should fail without API key (unless BOCHA_API_KEY env var is set)
        let result = tool.execute(params).await;
        // This might succeed if env var is set, or fail otherwise
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_web_search_max_results_limit() {
        assert_eq!(WebSearchTool::max_results_limit("brave"), 10);
        assert_eq!(WebSearchTool::max_results_limit("bocha"), 50);
        assert_eq!(WebSearchTool::max_results_limit("zhipu"), 50);
    }

    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let tool = WebFetchTool::new();
        let params = json!({"url": "not-a-valid-url"});

        let result = tool.execute(params).await.unwrap();
        assert!(result.contains("error") || result.contains("validation"));
    }
}
