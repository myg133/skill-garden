//! License File Service
//!
//! Reads and validates license files for enterprise private deployment mode.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// License file structure for enterprise deployments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    /// Maximum number of tenants allowed (None = unlimited)
    #[serde(default)]
    pub max_tenants: Option<i32>,
    /// Enabled features list (e.g., ["sso", "audit"])
    #[serde(default)]
    pub features: Vec<String>,
}

/// License service for reading file-based licenses
#[derive(Debug, Clone)]
pub struct LicenseService {
    license: Option<LicenseFile>,
    license_path: Option<PathBuf>,
}

impl Default for LicenseService {
    fn default() -> Self {
        Self::new()
    }
}

impl LicenseService {
    /// Create a new license service, reading from LICENSE_FILE env var if set
    pub fn new() -> Self {
        let license_path = std::env::var("LICENSE_FILE").ok().map(PathBuf::from);
        let license = license_path.as_ref().and_then(|path| {
            Self::load_license_file(path).ok()
        });
        
        Self {
            license,
            license_path,
        }
    }

    /// Create a license service with a specific path (for testing)
    pub fn from_path(path: &PathBuf) -> Self {
        let license = Self::load_license_file(path).ok();
        Self {
            license,
            license_path: Some(path.clone()),
        }
    }

    /// Load and parse a license file
    fn load_license_file(path: &PathBuf) -> Result<LicenseFile, LicenseError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LicenseError::FileReadError(e.to_string()))?;
        
        let license: LicenseFile = serde_json::from_str(&content)
            .map_err(|e| LicenseError::ParseError(e.to_string()))?;
        
        tracing::info!(
            "License file loaded: {} (max_tenants: {:?})",
            path.display(),
            license.max_tenants
        );
        
        Ok(license)
    }

    /// Check if a license is configured (LICENSE_FILE was set and file exists)
    pub fn is_configured(&self) -> bool {
        self.license.is_some()
    }

    /// Get the license path if configured
    pub fn license_path(&self) -> Option<&PathBuf> {
        self.license_path.as_ref()
    }

    /// Get max_tenants limit if configured
    pub fn max_tenants(&self) -> Option<i32> {
        self.license.as_ref().and_then(|l| l.max_tenants)
    }

    /// Check if a feature is enabled in the license
    pub fn has_feature(&self, feature: &str) -> bool {
        self.license
            .as_ref()
            .map(|l| l.features.contains(&feature.to_string()))
            .unwrap_or(false)
    }

    /// Check if tenant creation is allowed under current quota
    pub fn can_create_tenant(&self, current_count: i64) -> Result<(), LicenseQuotaExceeded> {
        if let Some(max) = self.max_tenants() {
            if current_count >= max as i64 {
                return Err(LicenseQuotaExceeded {
                    max_tenants: max,
                    current_count,
                });
            }
        }
        Ok(())
    }
}

/// Error types for license operations
#[derive(Debug, Clone)]
pub enum LicenseError {
    FileReadError(String),
    ParseError(String),
}

impl std::fmt::Display for LicenseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LicenseError::FileReadError(msg) => write!(f, "Failed to read license file: {}", msg),
            LicenseError::ParseError(msg) => write!(f, "Failed to parse license file: {}", msg),
        }
    }
}

impl std::error::Error for LicenseError {}

/// Error when tenant quota is exceeded
#[derive(Debug, Clone)]
pub struct LicenseQuotaExceeded {
    pub max_tenants: i32,
    pub current_count: i64,
}

impl std::fmt::Display for LicenseQuotaExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tenant quota exceeded: {} tenants allowed, {} tenants already exist",
            self.max_tenants, self.current_count
        )
    }
}

impl std::error::Error for LicenseQuotaExceeded {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_valid_license() {
        let temp_dir = TempDir::new().unwrap();
        let license_path = temp_dir.path().join("license.json");
        
        std::fs::write(&license_path, r#"{
            "max_tenants": 5,
            "features": ["sso", "audit"]
        }"#).unwrap();
        
        let service = LicenseService::from_path(&license_path);
        
        assert!(service.is_configured());
        assert_eq!(service.max_tenants(), Some(5));
        assert!(service.has_feature("sso"));
        assert!(service.has_feature("audit"));
        assert!(!service.has_feature("unknown"));
    }

    #[test]
    fn test_load_license_without_max_tenants() {
        let temp_dir = TempDir::new().unwrap();
        let license_path = temp_dir.path().join("license.json");
        
        std::fs::write(&license_path, r#"{
            "features": ["sso"]
        }"#).unwrap();
        
        let service = LicenseService::from_path(&license_path);
        
        assert!(service.is_configured());
        assert_eq!(service.max_tenants(), None);
    }

    #[test]
    fn test_load_empty_license() {
        let temp_dir = TempDir::new().unwrap();
        let license_path = temp_dir.path().join("license.json");
        
        std::fs::write(&license_path, "{}").unwrap();
        
        let service = LicenseService::from_path(&license_path);
        
        assert!(service.is_configured());
        assert_eq!(service.max_tenants(), None);
        assert!(!service.has_feature("anything"));
    }

    #[test]
    fn test_can_create_tenant_under_limit() {
        let temp_dir = TempDir::new().unwrap();
        let license_path = temp_dir.path().join("license.json");
        
        std::fs::write(&license_path, r#"{"max_tenants": 5}"#).unwrap();
        
        let service = LicenseService::from_path(&license_path);
        
        assert!(service.can_create_tenant(0).is_ok());
        assert!(service.can_create_tenant(4).is_ok());
    }

    #[test]
    fn test_can_create_tenant_at_limit() {
        let temp_dir = TempDir::new().unwrap();
        let license_path = temp_dir.path().join("license.json");
        
        std::fs::write(&license_path, r#"{"max_tenants": 5}"#).unwrap();
        
        let service = LicenseService::from_path(&license_path);
        
        let result = service.can_create_tenant(5);
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert_eq!(e.max_tenants, 5);
            assert_eq!(e.current_count, 5);
        }
    }

    #[test]
    fn test_can_create_tenant_over_limit() {
        let temp_dir = TempDir::new().unwrap();
        let license_path = temp_dir.path().join("license.json");
        
        std::fs::write(&license_path, r#"{"max_tenants": 5}"#).unwrap();
        
        let service = LicenseService::from_path(&license_path);
        
        let result = service.can_create_tenant(10);
        assert!(result.is_err());
    }

    #[test]
    fn test_unlimited_tenants() {
        let temp_dir = TempDir::new().unwrap();
        let license_path = temp_dir.path().join("license.json");
        
        // No max_tenants specified = unlimited
        std::fs::write(&license_path, r#"{"features": ["sso"]}"#).unwrap();
        
        let service = LicenseService::from_path(&license_path);
        
        assert!(service.can_create_tenant(0).is_ok());
        assert!(service.can_create_tenant(1000).is_ok());
    }

    #[test]
    fn test_invalid_license_file() {
        let temp_dir = TempDir::new().unwrap();
        let license_path = temp_dir.path().join("license.json");
        
        std::fs::write(&license_path, "not valid json").unwrap();
        
        let service = LicenseService::from_path(&license_path);
        
        assert!(!service.is_configured());
    }
}
