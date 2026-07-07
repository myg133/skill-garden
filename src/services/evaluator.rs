//! 评价服务 - 评价收集、置信度计算

use std::path::PathBuf;
use tokio::runtime::Handle;

use anyhow::Result;
use reqwest::Client;
use tracing::{debug, error, info};

use crate::db::repositories::evaluation::{EvaluationRepository, NewEvaluation as DbNewEvaluation};
use crate::models::error::AppError;
use crate::models::evaluation::{ErrorType, EvalTag, Evaluation, EvaluationResult, SkillStats};
use crate::schemas::validation::validate_evaluation_input;
use crate::services::storage::StorageService;
use crate::utils::RateLimiter;

/// 评价服务
#[derive(Debug, Clone)]
pub struct EvaluatorService {
    _storage: StorageService,
    rate_limiter: RateLimiter,
    eval_repo: EvaluationRepository,
    webhook_urls: Vec<String>,
    http_client: Client,
}

impl EvaluatorService {
    pub fn new(data_dir: PathBuf, eval_repo: EvaluationRepository) -> Self {
        // Support multiple webhook URLs via comma-separated env var
        let webhook_urls = std::env::var("AION_HIVE_EVAL_WEBHOOK_URLS")
            .map(|s| s.split(',').map(str::trim).map(String::from).collect())
            .unwrap_or_default();

        Self {
            _storage: StorageService::new(data_dir.clone()),
            rate_limiter: RateLimiter::default(),
            eval_repo,
            webhook_urls,
            http_client: Client::new(),
        }
    }

    /// Set webhook URLs for evaluation forwarding
    pub fn with_webhook_urls(mut self, urls: Vec<String>) -> Self {
        self.webhook_urls = urls;
        self
    }

    /// Add a single webhook URL
    pub fn add_webhook_url(mut self, url: String) -> Self {
        self.webhook_urls.push(url);
        self
    }

    /// Forward evaluation to all configured webhooks
    async fn forward_to_webhooks(&self, evaluation: &EvaluationResult) {
        if self.webhook_urls.is_empty() {
            return;
        }

        for webhook_url in &self.webhook_urls {
            match self
                .http_client
                .post(webhook_url)
                .json(evaluation)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    info!("Forwarded evaluation to webhook: {}", webhook_url);
                }
                Ok(resp) => {
                    error!(
                        "Webhook returned error status: {} for URL: {}",
                        resp.status(),
                        webhook_url
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to forward evaluation to webhook: {} - {}",
                        webhook_url, e
                    );
                }
            }
        }
    }

    /// 添加评价
    pub async fn add_evaluation(
        &self,
        skill_id: String,
        agent_id: String,
        success: bool,
        duration_ms: u64,
        error_type: Option<ErrorType>,
        tags: Vec<EvalTag>,
    ) -> Result<EvaluationResult, AppError> {
        validate_evaluation_input(&skill_id, duration_ms)?;

        let rate_key = format!("{}:{}", skill_id, agent_id);
        if !self.rate_limiter.check(&rate_key).await {
            return Err(AppError::EvaluationRateLimited);
        }

        let eval_db = DbNewEvaluation {
            skill_id: skill_id.clone(),
            agent_id: agent_id.clone(),
            success,
            duration_ms: duration_ms as i64,
            error_type: error_type.map(|e| format!("{:?}", e)),
            tags: tags.iter().map(|t| format!("{:?}", t)).collect(),
        };

        let evaluation = self
            .eval_repo
            .create(eval_db)
            .await
            .map_err(|e| AppError::from(e))?;
        let stats = self
            .eval_repo
            .get_stats(&skill_id)
            .await
            .map_err(|e| AppError::from(e))?;

        debug!(
            "Added evaluation for skill: {}, success: {}",
            skill_id, success
        );

        let result = EvaluationResult {
            success: true,
            evaluation_id: evaluation.id.to_string(),
            new_stats: SkillStats {
                skill_id: stats.skill_id,
                success_rate: stats.success_rate,
                avg_duration_ms: stats.avg_duration_ms as u64,
                total_evaluations: stats.total_evaluations as u32,
                unique_agents: stats.unique_agents as u32,
                confidence: stats.confidence,
                tags: stats.tags,
                local_version: None,
                latest_version: "1.0.0".to_string(),
                upgrade_available: false,
            },
        };

        // Forward to webhooks (if configured)
        self.forward_to_webhooks(&result).await;

        Ok(result)
    }

    /// 获取 Skill 统计
    pub fn get_stats(&self, skill_id: &str) -> Result<SkillStats, AppError> {
        let stats = Handle::current()
            .block_on(async { self.eval_repo.get_stats(skill_id).await })
            .map_err(|e| AppError::from(e))?;

        Ok(SkillStats {
            skill_id: stats.skill_id,
            success_rate: stats.success_rate,
            avg_duration_ms: stats.avg_duration_ms as u64,
            total_evaluations: stats.total_evaluations as u32,
            unique_agents: stats.unique_agents as u32,
            confidence: stats.confidence,
            tags: stats.tags,
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        })
    }

    /// 获取评价列表
    pub fn list_evaluations(&self, skill_id: &str) -> Result<Vec<Evaluation>, AppError> {
        let evals = Handle::current()
            .block_on(async { self.eval_repo.list_by_skill(skill_id, 100).await })
            .map_err(|e| AppError::from(e))?;

        Ok(evals
            .into_iter()
            .map(|e| Evaluation {
                id: e.id.to_string(),
                skill_id: e.skill_id,
                agent_id: e.agent_id,
                success: e.success,
                duration_ms: e.duration_ms as u64,
                error_type: e.error_type.and_then(|s| match s.as_str() {
                    "Timeout" => Some(ErrorType::Timeout),
                    "Crash" => Some(ErrorType::Crash),
                    "LogicError" => Some(ErrorType::LogicError),
                    _ => Some(ErrorType::Other),
                }),
                tags: e
                    .tags
                    .into_iter()
                    .filter_map(|s| match s.as_str() {
                        "Reliable" => Some(EvalTag::Reliable),
                        "Fast" => Some(EvalTag::Fast),
                        "Stable" => Some(EvalTag::Stable),
                        "Experimental" => Some(EvalTag::Experimental),
                        _ => None,
                    })
                    .collect(),
                timestamp: e.timestamp,
            })
            .collect())
    }

    /// 获取单条评价
    pub fn get_evaluation(&self, eval_id: uuid::Uuid) -> Result<Option<Evaluation>, AppError> {
        let eval = Handle::current()
            .block_on(async { self.eval_repo.find_by_id(eval_id).await })
            .map_err(|e| AppError::from(e))?;

        Ok(eval.map(|e| Evaluation {
            id: e.id.to_string(),
            skill_id: e.skill_id,
            agent_id: e.agent_id,
            success: e.success,
            duration_ms: e.duration_ms as u64,
            error_type: e.error_type.and_then(|s| match s.as_str() {
                "Timeout" => Some(ErrorType::Timeout),
                "Crash" => Some(ErrorType::Crash),
                "LogicError" => Some(ErrorType::LogicError),
                _ => Some(ErrorType::Other),
            }),
            tags: e
                .tags
                .into_iter()
                .filter_map(|s| match s.as_str() {
                    "Reliable" => Some(EvalTag::Reliable),
                    "Fast" => Some(EvalTag::Fast),
                    "Stable" => Some(EvalTag::Stable),
                    "Experimental" => Some(EvalTag::Experimental),
                    _ => None,
                })
                .collect(),
            timestamp: e.timestamp,
        }))
    }

    /// 删除评价
    pub fn delete_evaluation(&self, eval_id: uuid::Uuid) -> Result<(), AppError> {
        Handle::current()
            .block_on(async { self.eval_repo.delete_by_id(eval_id).await })
            .map_err(|e| AppError::from(e))
    }

    /// 获取当前 webhook URL 列表
    pub fn get_webhook_urls(&self) -> Vec<String> {
        self.webhook_urls.clone()
    }

    /// 添加 webhook URL
    pub fn add_webhook_url_dyn(&mut self, url: String) {
        if !self.webhook_urls.contains(&url) {
            self.webhook_urls.push(url);
        }
    }

    /// 移除 webhook URL
    pub fn remove_webhook_url(&mut self, index: usize) -> Result<(), AppError> {
        if index >= self.webhook_urls.len() {
            return Err(AppError::ValidationError(
                "Webhook index out of bounds".to_string(),
            ));
        }
        self.webhook_urls.remove(index);
        Ok(())
    }

    /// 获取评价剩余次数
    pub async fn get_remaining(&self, skill_id: &str, agent_id: &str) -> u32 {
        let rate_key = format!("{}:{}", skill_id, agent_id);
        self.rate_limiter.remaining(&rate_key).await
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        RateLimiter::new(crate::utils::RateLimitConfig::default())
    }
}

// Note: EvaluatorService tests require a PostgreSQL database.
// Integration tests in tests/ directory handle database-dependent testing.
