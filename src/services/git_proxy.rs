//! Git Proxy Service - Manages skill version control via Git Proxy API
//!
//! This service provides Git operations via Git Proxy API.
//! Skills are stored in Git repos and version-controlled through this service.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::models::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRef {
    pub name: String,
    pub commit: String,
    pub committed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFile {
    pub path: String,
    pub content: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    pub from_commit: String,
    pub to_commit: String,
    pub files_changed: Vec<String>,
    pub additions: u64,
    pub deletions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepo {
    pub id: String,
    pub name: String,
    pub clone_url: String,
    pub default_branch: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub url: String,
    pub events: Vec<String>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct GitProxyConfig {
    pub api_base: String,
    pub default_branch: String,
    pub timeout_seconds: u64,
}

impl Default for GitProxyConfig {
    fn default() -> Self {
        Self {
            api_base: std::env::var("GIT_PROXY_API_BASE")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            default_branch: "main".to_string(),
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitProxyService {
    client: Client,
    config: GitProxyConfig,
}

impl GitProxyService {
    pub fn new(config: GitProxyConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    pub fn with_default_config() -> Self {
        Self::new(GitProxyConfig::default())
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.config.api_base, path)
    }

    pub async fn list_branches(&self, repo_id: &str) -> Result<Vec<String>, AppError> {
        let url = self.api_url(&format!("/repos/{}/branches", repo_id));

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list branches: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::InternalError(format!(
                "Git Proxy API error: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct BranchResponse {
            name: String,
        }

        let branches: Vec<BranchResponse> = response.json().await
            .map_err(|e| AppError::InternalError(format!("Failed to parse branches: {}", e)))?;

        Ok(branches.into_iter().map(|b| b.name).collect())
    }

    pub async fn get_branches_with_refs(&self, repo_id: &str) -> Result<Vec<GitRef>, AppError> {
        let url = self.api_url(&format!("/repos/{}/branches", repo_id));

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get branches: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::InternalError(format!(
                "Git Proxy API error: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct BranchResponse {
            name: String,
            commit: CommitRef,
        }

        #[derive(Deserialize)]
        struct CommitRef {
            sha: String,
        }

        let branches: Vec<BranchResponse> = response.json().await
            .map_err(|e| AppError::InternalError(format!("Failed to parse branches: {}", e)))?;

        Ok(branches.into_iter().map(|b| GitRef {
            name: b.name,
            commit: b.commit.sha,
            committed_at: 0,
        }).collect())
    }

    pub async fn get_commits(
        &self,
        repo_id: &str,
        limit: u32,
    ) -> Result<Vec<GitRef>, AppError> {
        let url = self.api_url(&format!(
            "/repos/{}/commits?limit={}",
            repo_id,
            limit
        ));

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get commits: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::InternalError(format!(
                "Git Proxy API error: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct CommitResponse {
            sha: String,
            commit: CommitDetail,
        }

        #[derive(Deserialize)]
        struct CommitDetail {
            #[allow(dead_code)]
            message: String,
            author: AuthorDetail,
        }

        #[derive(Deserialize)]
        struct AuthorDetail {
            timestamp: String,
        }

        let commits: Vec<CommitResponse> = response.json().await
            .map_err(|e| AppError::InternalError(format!("Failed to parse commits: {}", e)))?;

        Ok(commits.into_iter().map(|c| {
            let timestamp = chrono::DateTime::parse_from_rfc3339(&c.commit.author.timestamp)
                .map(|dt| dt.timestamp())
                .unwrap_or(0);

            GitRef {
                name: c.sha[..7].to_string(),
                commit: c.sha,
                committed_at: timestamp,
            }
        }).collect())
    }

    pub async fn get_file_at_commit(
        &self,
        repo_id: &str,
        path: &str,
        commit: &str,
    ) -> Result<GitFile, AppError> {
        let url = self.api_url(&format!(
            "/repos/{}/contents/{}?ref={}",
            repo_id,
            path,
            commit
        ));

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get file: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::InternalError(format!(
                "Git Proxy API error: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct FileResponse {
            path: String,
            content: String,
            size: u64,
        }

        let file: FileResponse = response.json().await
            .map_err(|e| AppError::InternalError(format!("Failed to parse file: {}", e)))?;

        Ok(GitFile {
            path: file.path,
            content: file.content,
            size: file.size,
        })
    }

    pub async fn get_diff(
        &self,
        repo_id: &str,
        from_commit: &str,
        to_commit: &str,
    ) -> Result<GitDiff, AppError> {
        let url = self.api_url(&format!(
            "/repos/{}/compare/{}...{}",
            repo_id,
            from_commit,
            to_commit
        ));

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get diff: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::InternalError(format!(
                "Git Proxy API error: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct DiffResponse {
            files: Vec<String>,
            additions: u64,
            deletions: u64,
        }

        let diff: DiffResponse = response.json().await
            .map_err(|e| AppError::InternalError(format!("Failed to parse diff: {}", e)))?;

        Ok(GitDiff {
            from_commit: from_commit.to_string(),
            to_commit: to_commit.to_string(),
            files_changed: diff.files,
            additions: diff.additions,
            deletions: diff.deletions,
        })
    }

    pub async fn validate_git_url(&self, git_url: &str) -> Result<bool, AppError> {
        if !git_url.starts_with("http://") && !git_url.starts_with("https://") {
            return Ok(false);
        }

        let url = self.api_url(&format!("/repos/validate?url={}", git_url));

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to validate URL: {}", e)))?;

        Ok(response.status().is_success())
    }

    pub async fn create_webhook(
        &self,
        repo_id: &str,
        callback_url: &str,
        events: Vec<String>,
    ) -> Result<Webhook, AppError> {
        let url = self.api_url(&format!("/repos/{}/hooks", repo_id));

        #[derive(Serialize)]
        struct CreateWebhookRequest {
            url: String,
            events: Vec<String>,
        }

        let response = self.client.post(&url)
            .json(&CreateWebhookRequest {
                url: callback_url.to_string(),
                events,
            })
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create webhook: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::InternalError(format!(
                "Git Proxy API error: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct WebhookResponse {
            id: String,
            url: String,
            events: Vec<String>,
            active: bool,
        }

        let webhook: WebhookResponse = response.json().await
            .map_err(|e| AppError::InternalError(format!("Failed to parse webhook: {}", e)))?;

        Ok(Webhook {
            id: webhook.id,
            url: webhook.url,
            events: webhook.events,
            active: webhook.active,
        })
    }

    pub async fn delete_webhook(
        &self,
        repo_id: &str,
        webhook_id: &str,
    ) -> Result<(), AppError> {
        let url = self.api_url(&format!(
            "/repos/{}/hooks/{}",
            repo_id,
            webhook_id
        ));

        let response = self.client.delete(&url)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete webhook: {}", e)))?;

        if !response.status().is_success() && response.status().as_u16() != 404 {
            return Err(AppError::InternalError(format!(
                "Git Proxy API error: {}",
                response.status()
            )));
        }

        Ok(())
    }

    pub async fn read_file(&self, repo_id: &str, path: &str) -> Result<GitFile, AppError> {
        self.get_file_at_commit(repo_id, path, &self.config.default_branch).await
    }

    pub async fn health_check(&self) -> Result<bool, AppError> {
        let url = self.api_url("/health");

        let response = self.client.get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("Git Proxy health check failed: {}", e)))?;

        Ok(response.status().is_success())
    }
}

impl Default for GitProxyService {
    fn default() -> Self {
        Self::with_default_config()
    }
}
