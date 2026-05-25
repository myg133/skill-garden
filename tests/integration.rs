//! Integration tests

use tempfile::TempDir;
use tokio;

mod common;

// ============================================================================
// SearchService Tests (file-based, no database required)
// ============================================================================

#[tokio::test]
async fn test_search_add_and_search() {
    let temp_dir = TempDir::new().unwrap();
    let search_dir = temp_dir.path().join("search");

    let search = aion_hive::SearchService::new(&search_dir).unwrap();

    let skill = aion_hive::models::skill::Skill {
        id: "skill-test-v1".to_string(),
        name: "test".to_string(),
        description: "A test skill for searching".to_string(),
        tags: vec!["test".to_string(), "search".to_string()],
        version: "1.0.0".to_string(),
        author_agent_id: "agent-1".to_string(),
        created: chrono::Utc::now(),
        updated: chrono::Utc::now(),
        compatibility: ">=1.0.0".to_string(),
        dependencies: vec![],
        content: "# Test Content for Searching".to_string(),
        install_count: 10,
    };

    search.add_skill(&skill).unwrap();

    let results = search.search("searching", None, 10).unwrap();
    assert!(!results.is_empty(), "Expected results for 'searching' query");
    assert_eq!(results[0].skill_id, "skill-test-v1");
}

#[tokio::test]
async fn test_search_with_tags() {
    let temp_dir = TempDir::new().unwrap();
    let search_dir = temp_dir.path().join("search");

    let search = aion_hive::SearchService::new(&search_dir).unwrap();

    let skill = aion_hive::models::skill::Skill {
        id: "skill-web-v1".to_string(),
        name: "web-scraper".to_string(),
        description: "Scrapes web pages".to_string(),
        tags: vec!["web".to_string(), "scraper".to_string()],
        version: "1.0.0".to_string(),
        author_agent_id: "agent-1".to_string(),
        created: chrono::Utc::now(),
        updated: chrono::Utc::now(),
        compatibility: ">=1.0.0".to_string(),
        dependencies: vec![],
        content: "# Web Scraper".to_string(),
        install_count: 5,
    };

    search.add_skill(&skill).unwrap();

    let results = search.search("scraper", Some(&["web".to_string()]), 10).unwrap();
    assert!(!results.is_empty());

    let results = search.search("nonexistent", Some(&["nonexistent".to_string()]), 10).unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_search_delete() {
    let temp_dir = TempDir::new().unwrap();
    let search_dir = temp_dir.path().join("search");

    let search = aion_hive::SearchService::new(&search_dir).unwrap();

    let skill = aion_hive::models::skill::Skill {
        id: "skill-del-v1".to_string(),
        name: "temp-skill".to_string(),
        description: "Will be deleted later".to_string(),
        tags: vec![],
        version: "1.0.0".to_string(),
        author_agent_id: "agent-1".to_string(),
        created: chrono::Utc::now(),
        updated: chrono::Utc::now(),
        compatibility: ">=1.0.0".to_string(),
        dependencies: vec![],
        content: "# Temporary Skill".to_string(),
        install_count: 0,
    };

    search.add_skill(&skill).unwrap();

    let results = search.search("temporary", None, 10).unwrap();
    assert!(!results.is_empty(), "Expected results before delete");

    search.delete_skill("skill-del-v1").unwrap();

    let results = search.search("temporary", None, 10).unwrap();
    assert!(results.is_empty(), "Expected no results after delete");
}

// ============================================================================
// Validation Tests (file-based, no database required)
// ============================================================================

#[tokio::test]
async fn test_validation() {
    use aion_hive::schemas::validation::*;

    assert!(validate_skill_name("browse").is_ok());
    assert!(validate_skill_name("web-scraper").is_ok());
    assert!(validate_skill_name("my_skill_v2").is_ok());

    assert!(validate_skill_name("").is_err());
    assert!(validate_skill_name("invalid name").is_err());
    assert!(validate_skill_name("invalid.name").is_err());

    assert!(validate_version("1.0.0").is_ok());
    assert!(validate_version("0.1.0").is_ok());

    assert!(validate_version("invalid").is_err());
    assert!(validate_version("1.0").is_err());
}

#[tokio::test]
async fn test_malicious_content_detection() {
    use aion_hive::schemas::validation::*;

    assert!(validate_skill_content("# Valid content\n## Section", "test").is_ok());

    assert!(validate_skill_content("<script>alert(1)</script>", "test").is_err());
    assert!(validate_skill_content("javascript:alert(1)", "test").is_err());
}

#[tokio::test]
async fn test_rate_limiter() {
    use aion_hive::utils::RateLimiter;
    use aion_hive::utils::RateLimitConfig;

    let limiter = RateLimiter::new(RateLimitConfig {
        max_per_window: 3,
        window_secs: 60,
    });

    assert!(limiter.check("test_key").await);
    assert!(limiter.check("test_key").await);
    assert!(limiter.check("test_key").await);

    assert!(!limiter.check("test_key").await);

    assert!(limiter.check("other_key").await);

    limiter.reset("test_key").await;
    assert!(limiter.check("test_key").await);
}

#[tokio::test]
async fn test_weight_calculation() {
    use aion_hive::models::Evaluation;
    use aion_hive::utils::weight::{calculate_weight, EvalContext};

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
    assert!((weight - 1.6).abs() < 0.001);
}

// ============================================================================
// Storage Tests (file-based, no database required)
// ============================================================================

#[tokio::test]
async fn test_storage_service() {
    let temp_dir = TempDir::new().unwrap();
    let storage = aion_hive::StorageService::new(temp_dir.path().to_path_buf());

    let test_path = temp_dir.path().join("test.json");
    let test_data = serde_json::json!({"key": "value"});

    storage.write_json(&test_path, &test_data).unwrap();
    let loaded: serde_json::Value = storage.read_json(&test_path).unwrap();

    assert_eq!(loaded, test_data);
}

// ============================================================================
// Note: RegistryService and EvaluatorService tests were removed because they
// require PostgreSQL database support. These tests will be restored when
// proper database test infrastructure is in place.
// ============================================================================