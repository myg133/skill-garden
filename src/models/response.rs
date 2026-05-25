//! 统一响应格式

use serde::{Deserialize, Serialize};

use super::error::ErrorCode;

/// 统一 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(code: ErrorCode, message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code,
                message,
                details: None,
            }),
        }
    }

    pub fn err_with_details(code: ErrorCode, message: String, details: serde_json::Value) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code,
                message,
                details: Some(details),
            }),
        }
    }
}

/// API 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub skills_count: u32,
}

impl HealthStatus {
    pub fn ok(skills_count: u32) -> Self {
        Self {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now(),
            skills_count,
        }
    }

    pub fn degraded(skills_count: u32) -> Self {
        Self {
            status: "degraded".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now(),
            skills_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok() {
        let response: ApiResponse<String> = ApiResponse::ok("test data".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test data".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_err() {
        let response: ApiResponse<String> = ApiResponse::err(
            ErrorCode::InvalidSkillName,
            "Invalid name".to_string(),
        );
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, ErrorCode::InvalidSkillName);
        assert_eq!(error.message, "Invalid name");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_api_response_err_with_details() {
        let details = serde_json::json!({"field": "name", "reason": "too long"});
        let response: ApiResponse<String> = ApiResponse::err_with_details(
            ErrorCode::ValidationError,
            "Validation failed".to_string(),
            details.clone(),
        );
        assert!(!response.success);
        let error = response.error.unwrap();
        assert_eq!(error.code, ErrorCode::ValidationError);
        assert_eq!(error.details, Some(details));
    }

    #[test]
    fn test_api_response_serde() {
        let response: ApiResponse<u32> = ApiResponse::ok(42);
        let json = serde_json::to_string(&response).unwrap();
        let parsed: ApiResponse<u32> = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.data, Some(42));
    }

    #[test]
    fn test_api_error_serde() {
        let error = ApiError {
            code: ErrorCode::SkillNotFound,
            message: "Skill not found".to_string(),
            details: None,
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"code\":\"SKILL_NOT_FOUND\""));
        let parsed: ApiError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.code, ErrorCode::SkillNotFound);
    }

    #[test]
    fn test_health_status_ok() {
        let status = HealthStatus::ok(5);
        assert_eq!(status.status, "ok");
        assert_eq!(status.skills_count, 5);
        assert!(!status.version.is_empty());
    }

    #[test]
    fn test_health_status_degraded() {
        let status = HealthStatus::degraded(0);
        assert_eq!(status.status, "degraded");
        assert_eq!(status.skills_count, 0);
    }

    #[test]
    fn test_health_status_serde() {
        let status = HealthStatus::ok(10);
        let json = serde_json::to_string(&status).unwrap();
        let parsed: HealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, "ok");
        assert_eq!(parsed.skills_count, 10);
    }
}
