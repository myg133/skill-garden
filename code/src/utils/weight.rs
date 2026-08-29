//! 置信度权重计算

use crate::models::evaluation::Evaluation;

/// 权重计算上下文
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// 是否有成功历史
    pub has_success_history: bool,
    /// 是否是最近的评价
    pub is_recent: bool,
    /// 是否与多数一致
    pub matches_majority: bool,
    /// 是否是唯一评价
    pub is_singleton: bool,
    /// 是否太快
    pub too_fast: bool,
    /// 是否太慢
    pub too_slow: bool,
}

/// 权重常量
pub struct WeightConfig;

impl WeightConfig {
    pub const BASE: f64 = 1.0;

    // 加分
    pub const SUCCESS_HISTORY_BONUS: f64 = 0.2;
    pub const RECENT_BONUS: f64 = 0.1;
    pub const MAJORITY_BONUS: f64 = 0.3;

    // 扣分
    pub const SINGLETON_PENALTY: f64 = 0.5;
    pub const TOO_FAST_PENALTY: f64 = 0.3;
    pub const TOO_SLOW_PENALTY: f64 = 0.2;

    // 阈值
    pub const TOO_FAST_MS: u64 = 1000; // < 1秒
    pub const TOO_SLOW_MULTIPLIER: f64 = 10.0; // > 10倍平均
    pub const RECENT_HOURS: i64 = 24; // 24小时内
}

/// 计算单条评价的权重
pub fn calculate_weight(_eval: &Evaluation, context: &EvalContext) -> f64 {
    let mut weight = WeightConfig::BASE;

    // 加分
    if context.has_success_history {
        weight += WeightConfig::SUCCESS_HISTORY_BONUS;
    }
    if context.is_recent {
        weight += WeightConfig::RECENT_BONUS;
    }
    if context.matches_majority {
        weight += WeightConfig::MAJORITY_BONUS;
    }

    // 扣分
    if context.is_singleton {
        weight -= WeightConfig::SINGLETON_PENALTY;
    }
    if context.too_fast {
        weight -= WeightConfig::TOO_FAST_PENALTY;
    }
    if context.too_slow {
        weight -= WeightConfig::TOO_SLOW_PENALTY;
    }

    weight.max(0.1) // 最小权重
}

/// 构建评估上下文
pub fn build_context(
    eval: &Evaluation,
    total_evals: usize,
    success_count: usize,
    avg_duration: u64,
    has_successful_history: bool,
) -> EvalContext {
    let now = chrono::Utc::now();
    let recent_threshold = now - chrono::Duration::hours(WeightConfig::RECENT_HOURS);

    EvalContext {
        has_success_history: has_successful_history,
        is_recent: eval.timestamp >= recent_threshold,
        matches_majority: total_evals > 1 && {
            let majority_success = success_count * 2 > total_evals;
            eval.success == majority_success
        },
        is_singleton: total_evals == 1,
        too_fast: eval.duration_ms < WeightConfig::TOO_FAST_MS,
        too_slow: avg_duration > 0
            && eval.duration_ms as f64 > avg_duration as f64 * WeightConfig::TOO_SLOW_MULTIPLIER,
    }
}

/// 计算加权统计数据
pub fn calculate_weighted_stats(evaluations: &[Evaluation]) -> (f64, u64, f64) {
    if evaluations.is_empty() {
        return (0.0, 0, 0.0);
    }

    // 按 agent_id 分组计算是否有成功历史
    let mut agent_success: HashMap<&str, bool> = HashMap::new();
    for eval in evaluations {
        agent_success
            .entry(eval.agent_id.as_str())
            .and_modify(|e| *e = *e || eval.success)
            .or_insert(eval.success);
    }

    // 计算平均执行时间
    let avg_duration: u64 =
        evaluations.iter().map(|e| e.duration_ms).sum::<u64>() / evaluations.len() as u64;

    // 计算成功数
    let success_count = evaluations.iter().filter(|e| e.success).count();
    let has_successful_history = evaluations.iter().any(|e| e.success);

    // 计算每条评价的权重
    let total_evals = evaluations.len();
    let mut total_weight = 0.0;
    let mut weighted_success = 0.0;

    for eval in evaluations {
        let context = build_context(
            eval,
            total_evals,
            success_count,
            avg_duration,
            has_successful_history,
        );
        let weight = calculate_weight(eval, &context);
        total_weight += weight;

        if eval.success {
            weighted_success += weight;
        }
    }

    let success_rate = if total_weight > 0.0 {
        weighted_success / total_weight
    } else {
        0.0
    };

    // 计算置信度
    let confidence = calculate_confidence(
        total_evals as u32,
        success_rate,
        agent_success.values().filter(|&&v| v).count() as u32,
    );

    (success_rate, avg_duration, confidence)
}

/// 计算置信度
pub fn calculate_confidence(total: u32, success_rate: f64, unique_success: u32) -> f64 {
    if total < 3 {
        0.3 // 低置信度
    } else if total < 10 {
        0.5 // 中等置信度
    } else if success_rate > 0.8 && unique_success >= 2 {
        0.9 // 高置信度
    } else if success_rate > 0.5 {
        0.7
    } else {
        0.4
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::evaluation::{ErrorType, EvalTag};

    #[test]
    fn test_weight_calculation() {
        let eval = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-1".to_string(),
            true,
            5000,
            None,
            vec![],
        );

        let context = EvalContext {
            has_success_history: true,
            is_recent: true,
            matches_majority: true,
            is_singleton: false,
            too_fast: false,
            too_slow: false,
        };

        let weight = calculate_weight(&eval, &context);
        assert_eq!(weight, 1.0 + 0.2 + 0.1 + 0.3); // 1.6
    }

    #[test]
    fn test_weight_calculation_minimum_weight() {
        let eval = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-1".to_string(),
            true,
            5000,
            None,
            vec![],
        );

        let context = EvalContext {
            has_success_history: false,
            is_recent: false,
            matches_majority: false,
            is_singleton: true,
            too_fast: true,
            too_slow: true,
        };

        let weight = calculate_weight(&eval, &context);
        assert_eq!(weight, 0.1); // minimum weight
    }

    #[test]
    fn test_weight_config_defaults() {
        assert_eq!(WeightConfig::BASE, 1.0);
        assert_eq!(WeightConfig::SUCCESS_HISTORY_BONUS, 0.2);
        assert_eq!(WeightConfig::RECENT_BONUS, 0.1);
        assert_eq!(WeightConfig::MAJORITY_BONUS, 0.3);
        assert_eq!(WeightConfig::SINGLETON_PENALTY, 0.5);
        assert_eq!(WeightConfig::TOO_FAST_PENALTY, 0.3);
        assert_eq!(WeightConfig::TOO_SLOW_PENALTY, 0.2);
    }

    #[test]
    fn test_calculate_weighted_stats_empty() {
        let stats = calculate_weighted_stats(&[]);
        assert_eq!(stats, (0.0, 0, 0.0));
    }

    #[test]
    fn test_calculate_weighted_stats_single_eval() {
        let eval = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-1".to_string(),
            true,
            1000,
            None,
            vec![EvalTag::Reliable],
        );

        let stats = calculate_weighted_stats(&[eval]);
        assert_eq!(stats.0, 1.0); // success rate
        assert_eq!(stats.1, 1000); // avg duration
        assert_eq!(stats.2, 0.3); // confidence (low for < 3 evals)
    }

    #[test]
    fn test_calculate_weighted_stats_multiple_evals() {
        let eval1 = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-1".to_string(),
            true,
            1000,
            None,
            vec![],
        );
        let eval2 = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-2".to_string(),
            true,
            2000,
            None,
            vec![],
        );
        let eval3 = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-3".to_string(),
            false,
            3000,
            Some(ErrorType::Timeout),
            vec![],
        );

        let stats = calculate_weighted_stats(&[eval1, eval2, eval3]);
        assert!(stats.0 > 0.0 && stats.0 < 1.0); // success rate between 0 and 1
        assert_eq!(stats.1, 2000); // average duration
    }

    #[test]
    fn test_calculate_confidence() {
        assert_eq!(calculate_confidence(2, 0.5, 1), 0.3); // < 3, low
        assert_eq!(calculate_confidence(5, 0.5, 1), 0.5); // < 10, medium
        assert_eq!(calculate_confidence(15, 0.9, 3), 0.9); // > 10, > 0.8, >= 2 unique success
        assert_eq!(calculate_confidence(15, 0.6, 2), 0.7); // > 10, > 0.5
        assert_eq!(calculate_confidence(15, 0.3, 1), 0.4); // low success rate
    }

    #[test]
    fn test_build_context_singleton() {
        let eval = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-1".to_string(),
            true,
            5000,
            None,
            vec![],
        );

        let context = build_context(&eval, 1, 1, 1000, false);
        assert!(context.is_singleton);
    }

    #[test]
    fn test_build_context_matches_majority() {
        let eval = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-1".to_string(),
            true,
            5000,
            None,
            vec![],
        );

        let context = build_context(&eval, 4, 3, 1000, true);
        assert!(context.matches_majority); // majority is success (3 out of 4)
    }

    #[test]
    fn test_build_context_too_fast() {
        let eval = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-1".to_string(),
            true,
            500, // < 1000ms threshold
            None,
            vec![],
        );

        let context = build_context(&eval, 2, 1, 1000, false);
        assert!(context.too_fast);
    }

    #[test]
    fn test_build_context_too_slow() {
        let eval = Evaluation::new(
            "skill-test-v1".to_string(),
            "agent-1".to_string(),
            true,
            50000, // > 10 * 1000ms threshold
            None,
            vec![],
        );

        let context = build_context(&eval, 2, 1, 1000, false);
        assert!(context.too_slow);
    }

    #[test]
    fn test_eval_context_default() {
        let context = EvalContext::default();
        assert!(!context.has_success_history);
        assert!(!context.is_recent);
        assert!(!context.matches_majority);
        assert!(!context.is_singleton);
        assert!(!context.too_fast);
        assert!(!context.too_slow);
    }
}
