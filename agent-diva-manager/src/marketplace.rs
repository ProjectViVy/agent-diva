//! skills.sh marketplace adapter.
//!
//! Talks to the public skills.sh directory API that powers the `npx skills`
//! CLI: search lives at `GET /api/search` and installable file snapshots at
//! `GET /api/download/{owner}/{repo}/{slug}`. Snapshots are plain JSON
//! (`{ files: [{ path, contents }], hash }`), so no git or Node toolchain is
//! required on the gateway host.

use std::time::Duration;

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

pub const DEFAULT_MARKETPLACE_BASE_URL: &str = "https://skills.sh";
/// Optional override for the marketplace base URL (operators/tests).
pub const MARKETPLACE_BASE_URL_ENV: &str = "AGENT_DIVA_SKILLS_MARKETPLACE_URL";
const DEFAULT_SEARCH_LIMIT: u32 = 20;
const MAX_SEARCH_LIMIT: u32 = 50;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// One searchable entry from the skills.sh directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceSkill {
    /// Fully qualified id in the form `owner/repo/slug`.
    pub id: String,
    /// Skill name/slug as listed by the directory.
    pub name: String,
    /// Source repository in the form `owner/repo`.
    pub source: String,
    /// Reported install count.
    pub installs: u64,
}

#[derive(Debug, Deserialize)]
struct SearchApiResponse {
    #[serde(default)]
    skills: Vec<RawMarketplaceSkill>,
}

#[derive(Debug, Deserialize)]
struct RawMarketplaceSkill {
    id: String,
    name: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    installs: u64,
}

/// A downloaded skill snapshot: every file with its UTF-8 contents.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillSnapshot {
    #[serde(default)]
    pub files: Vec<SnapshotFile>,
    #[serde(default)]
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotFile {
    pub path: String,
    pub contents: String,
}

#[derive(Debug, Deserialize)]
struct UpstreamErrorBody {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// HTTP client for the skills.sh marketplace.
#[derive(Clone)]
pub struct MarketplaceClient {
    base_url: String,
    client: reqwest::Client,
}

impl MarketplaceClient {
    pub fn new() -> anyhow::Result<Self> {
        let base_url = std::env::var(MARKETPLACE_BASE_URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_MARKETPLACE_BASE_URL.to_string());
        Self::with_base_url(&base_url)
    }

    pub fn with_base_url(base_url: &str) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .user_agent(concat!("agent-diva/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to build marketplace HTTP client")?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        })
    }

    /// Search the directory. `query` must be at least 2 characters, matching
    /// the upstream API contract.
    pub async fn search(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<MarketplaceSkill>> {
        let limit = limit
            .unwrap_or(DEFAULT_SEARCH_LIMIT)
            .clamp(1, MAX_SEARCH_LIMIT);
        let url = format!("{}/api/search", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[("q", query), ("limit", &limit.to_string())])
            .send()
            .await
            .context("marketplace search request failed")?;
        let body = upstream_body(response, "marketplace search").await?;
        let parsed: SearchApiResponse =
            serde_json::from_slice(&body).context("marketplace search returned invalid JSON")?;
        Ok(parsed
            .skills
            .into_iter()
            .map(|raw| MarketplaceSkill {
                id: raw.id,
                name: raw.name,
                source: raw.source,
                installs: raw.installs,
            })
            .collect())
    }

    /// Download the file snapshot for one skill.
    pub async fn download(
        &self,
        owner: &str,
        repo: &str,
        slug: &str,
    ) -> anyhow::Result<SkillSnapshot> {
        let url = format!(
            "{}/api/download/{}/{}/{}",
            self.base_url,
            encode_segment(owner)?,
            encode_segment(repo)?,
            encode_segment(slug)?
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("marketplace download request failed")?;
        let body = upstream_body(response, "marketplace download").await?;
        serde_json::from_slice(&body).context("marketplace download returned invalid JSON")
    }
}

/// Split a fully qualified skill id (`owner/repo/slug`) into its parts.
pub fn parse_skill_id(id: &str) -> anyhow::Result<(String, String, String)> {
    let mut parts = id.split('/');
    let owner = parts.next().unwrap_or("");
    let repo = parts.next().unwrap_or("");
    let slug = parts.next().unwrap_or("");
    if owner.is_empty() || repo.is_empty() || slug.is_empty() || parts.next().is_some() {
        return Err(anyhow!(
            "marketplace skill id must look like owner/repo/slug, got {id:?}"
        ));
    }
    Ok((owner.to_string(), repo.to_string(), slug.to_string()))
}

fn encode_segment(segment: &str) -> anyhow::Result<String> {
    let valid = segment
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.');
    if !valid || segment.is_empty() || segment == "." || segment == ".." {
        return Err(anyhow!("invalid marketplace path segment: {segment:?}"));
    }
    Ok(segment.to_string())
}

async fn upstream_body(response: reqwest::Response, operation: &str) -> anyhow::Result<Vec<u8>> {
    let status = response.status();
    let body = response
        .bytes()
        .await
        .with_context(|| format!("{operation} response body read failed"))?
        .to_vec();
    if status.is_success() {
        return Ok(body);
    }
    let detail = serde_json::from_slice::<UpstreamErrorBody>(&body)
        .ok()
        .and_then(|parsed| parsed.error.or(parsed.message))
        .unwrap_or_else(|| String::from_utf8_lossy(&body).chars().take(200).collect());
    Err(anyhow!(
        "{operation} failed with upstream status {}: {detail}",
        status.as_u16()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// System-level HTTP proxies on dev machines can intercept loopback
    /// connections to the mock server; exempt loopback for these tests.
    fn allow_loopback_proxy() {
        let existing = std::env::var("NO_PROXY").unwrap_or_default();
        if existing.contains("127.0.0.1") {
            return;
        }
        let merged = format!("127.0.0.1,localhost,{existing}")
            .trim_matches(',')
            .to_string();
        std::env::set_var("NO_PROXY", &merged);
        std::env::set_var("no_proxy", &merged);
    }

    #[tokio::test]
    async fn search_maps_directory_rows() {
        allow_loopback_proxy();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/search"))
            .and(query_param("q", "commit"))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "query": "commit",
                "skills": [
                    {
                        "id": "juliusbrussee/caveman/caveman-commit",
                        "skillId": "caveman-commit",
                        "name": "caveman-commit",
                        "installs": 311_998_u64,
                        "source": "juliusbrussee/caveman"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = MarketplaceClient::with_base_url(&server.uri()).unwrap();
        let skills = client.search("commit", None).await.unwrap();
        assert_eq!(
            skills,
            vec![MarketplaceSkill {
                id: "juliusbrussee/caveman/caveman-commit".to_string(),
                name: "caveman-commit".to_string(),
                source: "juliusbrussee/caveman".to_string(),
                installs: 311_998,
            }]
        );
    }

    #[tokio::test]
    async fn search_surfaces_upstream_error_message() {
        allow_loopback_proxy();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/search"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                serde_json::json!({ "error": "Query must be at least 2 characters" }),
            ))
            .mount(&server)
            .await;

        let client = MarketplaceClient::with_base_url(&server.uri()).unwrap();
        let error = client.search("x", None).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Query must be at least 2 characters"));
    }

    #[tokio::test]
    async fn download_parses_snapshot_files() {
        allow_loopback_proxy();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/download/juliusbrussee/caveman/caveman-commit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [
                    { "path": "SKILL.md", "contents": "---\nname: caveman-commit\n---\nbody" },
                    { "path": "README.md", "contents": "readme" }
                ],
                "hash": "abc123"
            })))
            .mount(&server)
            .await;

        let client = MarketplaceClient::with_base_url(&server.uri()).unwrap();
        let snapshot = client
            .download("juliusbrussee", "caveman", "caveman-commit")
            .await
            .unwrap();
        assert_eq!(snapshot.hash, "abc123");
        assert_eq!(snapshot.files.len(), 2);
        assert_eq!(snapshot.files[0].path, "SKILL.md");
    }

    #[test]
    fn parse_skill_id_requires_three_segments() {
        let (owner, repo, slug) = parse_skill_id("vercel-labs/skills/find-skills").unwrap();
        assert_eq!(
            (owner.as_str(), repo.as_str(), slug.as_str()),
            ("vercel-labs", "skills", "find-skills")
        );
        assert!(parse_skill_id("vercel-labs/skills").is_err());
        assert!(parse_skill_id("a/b/c/d").is_err());
        assert!(parse_skill_id("/b/c").is_err());
    }

    #[test]
    fn encode_segment_rejects_traversal() {
        assert!(encode_segment("..").is_err());
        assert!(encode_segment("a/b").is_err());
        assert!(encode_segment("a b").is_err());
        assert_eq!(encode_segment("caveman-commit").unwrap(), "caveman-commit");
    }
}
