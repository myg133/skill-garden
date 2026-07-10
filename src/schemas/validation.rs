//! 输入验证

use crate::models::error::AppError;

/// 验证常量
pub const MAX_SKILL_SIZE: usize = 1_000_000; // 1MB
pub const MAX_NAME_LENGTH: usize = 100;
pub const MAX_TAG_COUNT: usize = 10;
pub const MAX_DESCRIPTION_LENGTH: usize = 2000;
pub const MAX_CONTENT_LENGTH: usize = 500_000;

/// 恶意内容模式
/// 注意：`..` / `../` 不在列表中，由下方专门的路径穿越检查处理（需同时命中路径特征）
const MALICIOUS_PATTERNS: &[&str] = &[
    "<script",
    "javascript:",
    "onerror=",
    "onclick=",
    "onload=",
    "onmouseover=",
    "eval(",
    "innerHTML",
    "/etc/passwd",
    r"C:\Windows",
    "file://",
    "ftp://",
];

/// 验证 Skill 名称
pub fn validate_skill_name(name: &str) -> Result<(), AppError> {
    // 检查长度
    if name.is_empty() {
        return Err(AppError::InvalidSkillName(
            "Name cannot be empty".to_string(),
        ));
    }
    if name.len() > MAX_NAME_LENGTH {
        return Err(AppError::InvalidSkillName(format!(
            "Name too long: {} (max {})",
            name.len(),
            MAX_NAME_LENGTH
        )));
    }

    // 检查字符
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
    {
        return Err(AppError::InvalidSkillName(format!(
            "Invalid characters in '{}'. Only alphanumeric, hyphen, underscore, space allowed",
            name
        )));
    }

    Ok(())
}

/// 验证标签
pub fn validate_tags(tags: &[String]) -> Result<(), AppError> {
    if tags.len() > MAX_TAG_COUNT {
        return Err(AppError::TooManyTags(tags.len()));
    }

    for tag in tags {
        if tag.is_empty() || tag.len() > 50 {
            return Err(AppError::ValidationError(format!(
                "Invalid tag: '{}' (empty or too long)",
                tag
            )));
        }
        if !tag
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::ValidationError(format!(
                "Invalid tag characters: '{}'",
                tag
            )));
        }
    }

    Ok(())
}

/// 验证 Skill 内容
pub fn validate_skill_content(content: &str, _name: &str) -> Result<(), AppError> {
    // 检查大小
    let size = content.len();
    if size > MAX_SKILL_SIZE {
        return Err(AppError::SkillTooLarge(size));
    }

    if size > MAX_CONTENT_LENGTH {
        return Err(AppError::SkillInvalidFormat(format!(
            "Content too long: {} (max {})",
            size, MAX_CONTENT_LENGTH
        )));
    }

    // 检查恶意内容
    let content_lower = content.to_lowercase();
    for pattern in MALICIOUS_PATTERNS {
        if content_lower.contains(&pattern.to_lowercase()) {
            // 排除合理的 frontmatter 和 markdown 内容
            if pattern == &"<script" && content.contains("```") {
                continue; // 代码块中的 <script 可能是合法的
            }
            return Err(AppError::MaliciousContent);
        }
    }

    // 检查文件名路径遍历
    if content.contains("..") || content.contains("../") {
        if content.contains("/etc/") || content.contains("C:") {
            return Err(AppError::MaliciousContent);
        }
    }

    Ok(())
}

/// 验证版本号 (semver)
pub fn validate_version(version: &str) -> Result<(), AppError> {
    if version.is_empty() {
        return Err(AppError::InvalidVersion(
            "Version cannot be empty".to_string(),
        ));
    }

    // 简单的 semver 验证
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 3 {
        return Err(AppError::InvalidVersion(format!(
            "Invalid semver: '{}'. Expected x.y.z",
            version
        )));
    }

    for part in parts {
        if part.parse::<u32>().is_err() {
            return Err(AppError::InvalidVersion(format!(
                "Invalid version number: '{}' in '{}'",
                part, version
            )));
        }
    }

    Ok(())
}

/// 验证描述
pub fn validate_description(desc: &str) -> Result<(), AppError> {
    if desc.len() > MAX_DESCRIPTION_LENGTH {
        return Err(AppError::ValidationError(format!(
            "Description too long: {} (max {})",
            desc.len(),
            MAX_DESCRIPTION_LENGTH
        )));
    }

    Ok(())
}

/// 验证评价输入
pub fn validate_evaluation_input(skill_id: &str, duration_ms: u64) -> Result<(), AppError> {
    if skill_id.is_empty() {
        return Err(AppError::EvaluationInvalid(
            "skill_id cannot be empty".to_string(),
        ));
    }

    // 执行时间不能为0（除非是立即失败）
    if duration_ms == 0 {
        // 允许，但可能是异常
    }

    // 执行时间不能超过1小时
    if duration_ms > 3_600_000 {
        return Err(AppError::EvaluationInvalid(format!(
            "Duration too long: {}ms (max 1 hour)",
            duration_ms
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_skill_name() {
        assert!(validate_skill_name("browse").is_ok());
        assert!(validate_skill_name("web-scraper").is_ok());
        assert!(validate_skill_name("my_skill_v2").is_ok());
        assert!(validate_skill_name("Evolver Test Gene").is_ok());
        assert!(validate_skill_name("").is_err());
        assert!(validate_skill_name("invalid.name").is_err());
    }

    #[test]
    fn test_validate_tags() {
        assert!(validate_tags(&["web".to_string()]).is_ok());
        assert!(
            validate_tags(&["web".to_string(), "scraper".to_string(), "api".to_string()]).is_ok()
        );
        assert!(validate_tags(&[]).is_ok());
        assert!(validate_tags(&["a".repeat(51)]).is_err());
    }

    #[test]
    fn test_validate_version() {
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("0.1.0").is_ok());
        assert!(validate_version("1.0").is_err());
        assert!(validate_version("invalid").is_err());
    }

    #[test]
    fn test_validate_malicious_content() {
        assert!(validate_skill_content("# Valid content", "test").is_ok());
        assert!(validate_skill_content("<script>alert(1)</script>", "test").is_err());
        assert!(validate_skill_content("javascript:alert(1)", "test").is_err());
    }

    #[test]
    fn test_validate_skill_name_too_long() {
        let long_name = "a".repeat(101);
        let result = validate_skill_name(&long_name);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::InvalidSkillName(_)));
    }

    #[test]
    fn test_validate_skill_name_invalid_chars() {
        assert!(validate_skill_name("invalid.name").is_err());
        assert!(validate_skill_name("invalid/name").is_err());
        assert!(validate_skill_name("invalid@name").is_err());
    }

    #[test]
    fn test_validate_tags_too_many() {
        let many_tags: Vec<String> = (0..11).map(|i| format!("tag{}", i)).collect();
        let result = validate_tags(&many_tags);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::TooManyTags(11)));
    }

    #[test]
    fn test_validate_tags_invalid_chars() {
        assert!(validate_tags(&["tag with spaces".to_string()]).is_err());
        assert!(validate_tags(&["tag.with.dots".to_string()]).is_err());
    }

    #[test]
    fn test_validate_skill_content_too_large() {
        let large_content = "x".repeat(1_000_001);
        let result = validate_skill_content(&large_content, "test");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::SkillTooLarge(_)));
    }

    #[test]
    fn test_validate_skill_content_too_long() {
        let long_content = "x".repeat(500_001);
        let result = validate_skill_content(&long_content, "test");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::SkillInvalidFormat(_)
        ));
    }

    #[test]
    fn test_validate_skill_content_script_in_code_block() {
        let content = r#"
# Example

```javascript
<script>alert('xss')</script>
```

This is fine because it's in a code block.
"#;
        assert!(validate_skill_content(content, "test").is_ok());
    }

    #[test]
    fn test_validate_skill_content_path_traversal() {
        assert!(validate_skill_content("/etc/passwd content", "test").is_err());
        assert!(validate_skill_content("C:\\Windows\\system32", "test").is_err());
    }

    #[test]
    fn test_validate_version_empty() {
        assert!(validate_version("").is_err());
        assert!(matches!(
            validate_version("").unwrap_err(),
            AppError::InvalidVersion(_)
        ));
    }

    #[test]
    fn test_validate_version_invalid_number() {
        assert!(validate_version("1.0.invalid").is_err());
        assert!(validate_version("1.invalid.0").is_err());
    }

    #[test]
    fn test_validate_description_too_long() {
        let long_desc = "a".repeat(2001);
        let result = validate_description(&long_desc);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ValidationError(_)));
    }

    #[test]
    fn test_validate_description_ok() {
        assert!(validate_description("Short description").is_ok());
    }

    #[test]
    fn test_validate_evaluation_input_empty_skill_id() {
        let result = validate_evaluation_input("", 1000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::EvaluationInvalid(_)
        ));
    }

    #[test]
    fn test_validate_evaluation_input_duration_zero() {
        assert!(validate_evaluation_input("skill-1", 0).is_ok());
    }

    #[test]
    fn test_validate_evaluation_input_duration_too_long() {
        let result = validate_evaluation_input("skill-1", 3_600_001);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::EvaluationInvalid(_)
        ));
    }

    #[test]
    fn test_validate_evaluation_input_ok() {
        assert!(validate_evaluation_input("skill-1", 5000).is_ok());
    }
}
