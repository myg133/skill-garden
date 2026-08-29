//! Evaluation 数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    Timeout,
    Crash,
    LogicError,
    Other,
}

/// 评价标签
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalTag {
    Reliable,
    Fast,
    Stable,
    Experimental,
}

/// 单条评价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub id: String,
    pub skill_id: String,
    pub agent_id: String,
    pub success: bool,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<ErrorType>,
    #[serde(default)]
    pub tags: Vec<EvalTag>,
    pub timestamp: DateTime<Utc>,
}

impl Evaluation {
    pub fn new(
        skill_id: String,
        agent_id: String,
        success: bool,
        duration_ms: u64,
        error_type: Option<ErrorType>,
        tags: Vec<EvalTag>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            skill_id,
            agent_id,
            success,
            duration_ms,
            error_type,
            tags,
            timestamp: Utc::now(),
        }
    }
}

/// 评价文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationFile {
    pub skill_id: String,
    pub evaluations: Vec<Evaluation>,
}

impl EvaluationFile {
    pub fn new(skill_id: String) -> Self {
        Self {
            skill_id,
            evaluations: Vec::new(),
        }
    }
}

/// 置信度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
}

/// Skill 统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStats {
    pub skill_id: String,
    /// 加权成功率 (0-1)
    pub success_rate: f64,
    /// 加权平均执行时间 (ms)
    pub avg_duration_ms: u64,
    /// 总评价数
    pub total_evaluations: u32,
    /// 评价过的唯一 Agent 数
    pub unique_agents: u32,
    /// 置信度 (0-1)
    pub confidence: f64,
    /// 聚合后的高频标签
    pub tags: Vec<String>,
    /// Agent 本地版本（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_version: Option<String>,
    /// 最新版本
    pub latest_version: String,
    /// 是否有新版本
    pub upgrade_available: bool,
}

impl SkillStats {
    pub fn confidence_level(&self) -> ConfidenceLevel {
        if self.total_evaluations < 3 {
            ConfidenceLevel::Low
        } else if self.total_evaluations > 10 && self.success_rate > 0.8 {
            ConfidenceLevel::High
        } else {
            ConfidenceLevel::Medium
        }
    }
}

impl Default for SkillStats {
    fn default() -> Self {
        Self {
            skill_id: String::new(),
            success_rate: 0.0,
            avg_duration_ms: 0,
            total_evaluations: 0,
            unique_agents: 0,
            confidence: 0.0,
            tags: Vec::new(),
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        }
    }
}

/// 评价结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub success: bool,
    pub evaluation_id: String,
    pub new_stats: SkillStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_serde() {
        let timeout = ErrorType::Timeout;
        let json = serde_json::to_string(&timeout).unwrap();
        assert_eq!(json, "\"timeout\"");
        let parsed: ErrorType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ErrorType::Timeout);
    }

    #[test]
    fn test_eval_tag_serde() {
        let reliable = EvalTag::Reliable;
        let json = serde_json::to_string(&reliable).unwrap();
        assert_eq!(json, "\"reliable\"");
        let parsed: EvalTag = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, EvalTag::Reliable);
    }

    #[test]
    fn test_evaluation_new() {
        let eval = Evaluation::new(
            "skill-test".to_string(),
            "agent-1".to_string(),
            true,
            5000,
            None,
            vec![EvalTag::Reliable],
        );
        assert!(!eval.id.is_empty());
        assert_eq!(eval.skill_id, "skill-test");
        assert_eq!(eval.agent_id, "agent-1");
        assert!(eval.success);
        assert_eq!(eval.duration_ms, 5000);
        assert!(eval.error_type.is_none());
        assert_eq!(eval.tags, vec![EvalTag::Reliable]);
    }

    #[test]
    fn test_evaluation_with_error_type() {
        let eval = Evaluation::new(
            "skill-test".to_string(),
            "agent-1".to_string(),
            false,
            30000,
            Some(ErrorType::Timeout),
            vec![],
        );
        assert!(!eval.success);
        assert_eq!(eval.error_type, Some(ErrorType::Timeout));
    }

    #[test]
    fn test_evaluation_file_new() {
        let eval_file = EvaluationFile::new("skill-test".to_string());
        assert_eq!(eval_file.skill_id, "skill-test");
        assert!(eval_file.evaluations.is_empty());
    }

    #[test]
    fn test_skill_stats_confidence_level() {
        let low_stats = SkillStats {
            skill_id: "test".to_string(),
            success_rate: 0.5,
            avg_duration_ms: 100,
            total_evaluations: 2,
            unique_agents: 1,
            confidence: 0.3,
            tags: vec![],
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        };
        assert_eq!(low_stats.confidence_level(), ConfidenceLevel::Low);

        let high_stats = SkillStats {
            skill_id: "test".to_string(),
            success_rate: 0.9,
            avg_duration_ms: 100,
            total_evaluations: 15,
            unique_agents: 3,
            confidence: 0.9,
            tags: vec![],
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        };
        assert_eq!(high_stats.confidence_level(), ConfidenceLevel::High);

        let medium_stats = SkillStats {
            skill_id: "test".to_string(),
            success_rate: 0.6,
            avg_duration_ms: 100,
            total_evaluations: 5,
            unique_agents: 2,
            confidence: 0.5,
            tags: vec![],
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        };
        assert_eq!(medium_stats.confidence_level(), ConfidenceLevel::Medium);
    }

    #[test]
    fn test_skill_stats_default() {
        let stats = SkillStats::default();
        assert!(stats.skill_id.is_empty());
        assert_eq!(stats.success_rate, 0.0);
        assert_eq!(stats.total_evaluations, 0);
        assert_eq!(stats.latest_version, "1.0.0");
    }

    #[test]
    fn test_confidence_level_serde() {
        let level = ConfidenceLevel::High;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"high\"");
        let parsed: ConfidenceLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ConfidenceLevel::High);
    }
}
