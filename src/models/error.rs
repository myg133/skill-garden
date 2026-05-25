//! 错误类型定义

use thiserror::Error;
use serde::{Deserialize, Serialize};

/// 统一错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // 通用
    Unknown,
    InternalError,

    // Skill 相关
    SkillNotFound,
    SkillAlreadyExists,
    SkillInstallFailed,
    SkillCreateFailed,
    SkillUpdateFailed,
    SkillInvalidFormat,
    SkillTooLarge,
    MaliciousContent,
    InvalidSkillName,
    TooManyTags,

    // Evaluation 相关
    EvaluationInvalid,
    EvaluationRateLimited,

    // Storage 相关
    RegistryReadFailed,
    RegistryWriteFailed,
    RegistryLockFailed,
    FileNotFound,

    // 验证相关
    ValidationError,
    InvalidVersion,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::Unknown => write!(f, "UNKNOWN"),
            ErrorCode::InternalError => write!(f, "INTERNAL_ERROR"),
            ErrorCode::SkillNotFound => write!(f, "SKILL_NOT_FOUND"),
            ErrorCode::SkillAlreadyExists => write!(f, "SKILL_ALREADY_EXISTS"),
            ErrorCode::SkillInstallFailed => write!(f, "SKILL_INSTALL_FAILED"),
            ErrorCode::SkillCreateFailed => write!(f, "SKILL_CREATE_FAILED"),
            ErrorCode::SkillUpdateFailed => write!(f, "SKILL_UPDATE_FAILED"),
            ErrorCode::SkillInvalidFormat => write!(f, "SKILL_INVALID_FORMAT"),
            ErrorCode::SkillTooLarge => write!(f, "SKILL_TOO_LARGE"),
            ErrorCode::MaliciousContent => write!(f, "MALICIOUS_CONTENT"),
            ErrorCode::InvalidSkillName => write!(f, "INVALID_SKILL_NAME"),
            ErrorCode::TooManyTags => write!(f, "TOO_MANY_TAGS"),
            ErrorCode::EvaluationInvalid => write!(f, "EVALUATION_INVALID"),
            ErrorCode::EvaluationRateLimited => write!(f, "EVALUATION_RATE_LIMITED"),
            ErrorCode::RegistryReadFailed => write!(f, "REGISTRY_READ_FAILED"),
            ErrorCode::RegistryWriteFailed => write!(f, "REGISTRY_WRITE_FAILED"),
            ErrorCode::RegistryLockFailed => write!(f, "REGISTRY_LOCK_FAILED"),
            ErrorCode::FileNotFound => write!(f, "FILE_NOT_FOUND"),
            ErrorCode::ValidationError => write!(f, "VALIDATION_ERROR"),
            ErrorCode::InvalidVersion => write!(f, "INVALID_VERSION"),
        }
    }
}

/// 应用错误
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Skill already exists: {0}")]
    SkillAlreadyExists(String),

    #[error("Skill install failed: {0}")]
    SkillInstallFailed(String),

    #[error("Skill create failed: {0}")]
    SkillCreateFailed(String),

    #[error("Skill update failed: {0}")]
    SkillUpdateFailed(String),

    #[error("Invalid skill format: {0}")]
    SkillInvalidFormat(String),

    #[error("Skill too large: {0} bytes (max 1MB)")]
    SkillTooLarge(usize),

    #[error("Malicious content detected")]
    MaliciousContent,

    #[error("Invalid skill name: {0}")]
    InvalidSkillName(String),

    #[error("Too many tags: {0} (max 10)")]
    TooManyTags(usize),

    #[error("Invalid evaluation: {0}")]
    EvaluationInvalid(String),

    #[error("Evaluation rate limited")]
    EvaluationRateLimited,

    #[error("Registry read failed: {0}")]
    RegistryReadFailed(String),

    #[error("Registry write failed: {0}")]
    RegistryWriteFailed(String),

    #[error("Registry lock failed: {0}")]
    RegistryLockFailed(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl AppError {
    pub fn code(&self) -> ErrorCode {
        match self {
            AppError::SkillNotFound(_) => ErrorCode::SkillNotFound,
            AppError::SkillAlreadyExists(_) => ErrorCode::SkillAlreadyExists,
            AppError::SkillInstallFailed(_) => ErrorCode::SkillInstallFailed,
            AppError::SkillCreateFailed(_) => ErrorCode::SkillCreateFailed,
            AppError::SkillUpdateFailed(_) => ErrorCode::SkillUpdateFailed,
            AppError::SkillInvalidFormat(_) => ErrorCode::SkillInvalidFormat,
            AppError::SkillTooLarge(_) => ErrorCode::SkillTooLarge,
            AppError::MaliciousContent => ErrorCode::MaliciousContent,
            AppError::InvalidSkillName(_) => ErrorCode::InvalidSkillName,
            AppError::TooManyTags(_) => ErrorCode::TooManyTags,
            AppError::EvaluationInvalid(_) => ErrorCode::EvaluationInvalid,
            AppError::EvaluationRateLimited => ErrorCode::EvaluationRateLimited,
            AppError::RegistryReadFailed(_) => ErrorCode::RegistryReadFailed,
            AppError::RegistryWriteFailed(_) => ErrorCode::RegistryWriteFailed,
            AppError::RegistryLockFailed(_) => ErrorCode::RegistryLockFailed,
            AppError::FileNotFound(_) => ErrorCode::FileNotFound,
            AppError::ValidationError(_) => ErrorCode::ValidationError,
            AppError::InvalidVersion(_) => ErrorCode::InvalidVersion,
            AppError::InternalError(_) => ErrorCode::InternalError,
        }
    }
}

// 实现 serde::Serialize
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// 实现 From<std::io::Error>
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::RegistryReadFailed(err.to_string())
    }
}

// 实现 From<anyhow::Error>
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::InternalError(err.to_string())
    }
}

// 实现 From<serde_json::Error>
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::ValidationError(err.to_string())
    }
}

// 实现 From<tantivy::TantivyError>
impl From<tantivy::TantivyError> for AppError {
    fn from(err: tantivy::TantivyError) -> Self {
        AppError::InternalError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::Unknown.to_string(), "UNKNOWN");
        assert_eq!(ErrorCode::InternalError.to_string(), "INTERNAL_ERROR");
        assert_eq!(ErrorCode::SkillNotFound.to_string(), "SKILL_NOT_FOUND");
        assert_eq!(ErrorCode::SkillAlreadyExists.to_string(), "SKILL_ALREADY_EXISTS");
        assert_eq!(ErrorCode::SkillInstallFailed.to_string(), "SKILL_INSTALL_FAILED");
        assert_eq!(ErrorCode::SkillCreateFailed.to_string(), "SKILL_CREATE_FAILED");
        assert_eq!(ErrorCode::SkillUpdateFailed.to_string(), "SKILL_UPDATE_FAILED");
        assert_eq!(ErrorCode::SkillInvalidFormat.to_string(), "SKILL_INVALID_FORMAT");
        assert_eq!(ErrorCode::SkillTooLarge.to_string(), "SKILL_TOO_LARGE");
        assert_eq!(ErrorCode::MaliciousContent.to_string(), "MALICIOUS_CONTENT");
        assert_eq!(ErrorCode::InvalidSkillName.to_string(), "INVALID_SKILL_NAME");
        assert_eq!(ErrorCode::TooManyTags.to_string(), "TOO_MANY_TAGS");
        assert_eq!(ErrorCode::EvaluationInvalid.to_string(), "EVALUATION_INVALID");
        assert_eq!(ErrorCode::EvaluationRateLimited.to_string(), "EVALUATION_RATE_LIMITED");
        assert_eq!(ErrorCode::RegistryReadFailed.to_string(), "REGISTRY_READ_FAILED");
        assert_eq!(ErrorCode::RegistryWriteFailed.to_string(), "REGISTRY_WRITE_FAILED");
        assert_eq!(ErrorCode::RegistryLockFailed.to_string(), "REGISTRY_LOCK_FAILED");
        assert_eq!(ErrorCode::FileNotFound.to_string(), "FILE_NOT_FOUND");
        assert_eq!(ErrorCode::ValidationError.to_string(), "VALIDATION_ERROR");
        assert_eq!(ErrorCode::InvalidVersion.to_string(), "INVALID_VERSION");
    }

    #[test]
    fn test_app_error_display() {
        assert_eq!(
            AppError::SkillNotFound("test-id".to_string()).to_string(),
            "Skill not found: test-id"
        );
        assert_eq!(
            AppError::SkillAlreadyExists("test-skill".to_string()).to_string(),
            "Skill already exists: test-skill"
        );
        assert_eq!(
            AppError::SkillTooLarge(1024).to_string(),
            "Skill too large: 1024 bytes (max 1MB)"
        );
        assert_eq!(
            AppError::MaliciousContent.to_string(),
            "Malicious content detected"
        );
        assert_eq!(
            AppError::TooManyTags(15).to_string(),
            "Too many tags: 15 (max 10)"
        );
    }

    #[test]
    fn test_app_error_code() {
        assert_eq!(AppError::SkillNotFound("x".to_string()).code(), ErrorCode::SkillNotFound);
        assert_eq!(AppError::SkillAlreadyExists("x".to_string()).code(), ErrorCode::SkillAlreadyExists);
        assert_eq!(AppError::SkillInstallFailed("x".to_string()).code(), ErrorCode::SkillInstallFailed);
        assert_eq!(AppError::SkillCreateFailed("x".to_string()).code(), ErrorCode::SkillCreateFailed);
        assert_eq!(AppError::SkillUpdateFailed("x".to_string()).code(), ErrorCode::SkillUpdateFailed);
        assert_eq!(AppError::SkillInvalidFormat("x".to_string()).code(), ErrorCode::SkillInvalidFormat);
        assert_eq!(AppError::SkillTooLarge(0).code(), ErrorCode::SkillTooLarge);
        assert_eq!(AppError::MaliciousContent.code(), ErrorCode::MaliciousContent);
        assert_eq!(AppError::InvalidSkillName("x".to_string()).code(), ErrorCode::InvalidSkillName);
        assert_eq!(AppError::TooManyTags(0).code(), ErrorCode::TooManyTags);
        assert_eq!(AppError::EvaluationInvalid("x".to_string()).code(), ErrorCode::EvaluationInvalid);
        assert_eq!(AppError::EvaluationRateLimited.code(), ErrorCode::EvaluationRateLimited);
        assert_eq!(AppError::RegistryReadFailed("x".to_string()).code(), ErrorCode::RegistryReadFailed);
        assert_eq!(AppError::RegistryWriteFailed("x".to_string()).code(), ErrorCode::RegistryWriteFailed);
        assert_eq!(AppError::RegistryLockFailed("x".to_string()).code(), ErrorCode::RegistryLockFailed);
        assert_eq!(AppError::FileNotFound("x".to_string()).code(), ErrorCode::FileNotFound);
        assert_eq!(AppError::ValidationError("x".to_string()).code(), ErrorCode::ValidationError);
        assert_eq!(AppError::InvalidVersion("x".to_string()).code(), ErrorCode::InvalidVersion);
        assert_eq!(AppError::InternalError("x".to_string()).code(), ErrorCode::InternalError);
    }

    #[test]
    fn test_app_error_serialize() {
        let err = AppError::SkillNotFound("test-id".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Skill not found"));
    }

    #[test]
    fn test_error_code_serde() {
        let code = ErrorCode::SkillNotFound;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "\"SKILL_NOT_FOUND\"");
        let decoded: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, ErrorCode::SkillNotFound);
    }
}
