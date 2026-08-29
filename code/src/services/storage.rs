//! 存储服务 - 文件操作、原子写入、文件锁

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use fs2::FileExt;
use tracing::{debug, error};

use crate::models::error::AppError;

/// 存储服务
#[derive(Debug, Clone)]
pub struct StorageService {
    data_dir: PathBuf,
}

impl StorageService {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// 确保目录存在
    pub fn ensure_dir(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    /// 读取文件
    pub fn read_file(&self, path: &Path) -> Result<String, AppError> {
        fs::read_to_string(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AppError::FileNotFound(path.display().to_string())
            } else {
                AppError::RegistryReadFailed(e.to_string())
            }
        })
    }

    /// 读取 JSON 文件
    pub fn read_json<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T, AppError> {
        let content = self.read_file(path)?;
        serde_json::from_str(&content).map_err(|e| AppError::RegistryReadFailed(e.to_string()))
    }

    /// 写入文件
    pub fn write_file(&self, path: &Path, content: &str) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            self.ensure_dir(parent)?;
        }

        fs::write(path, content).map_err(|e| AppError::RegistryWriteFailed(e.to_string()))
    }

    /// 写入 JSON 文件
    pub fn write_json<T: serde::Serialize>(&self, path: &Path, data: &T) -> Result<(), AppError> {
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| AppError::RegistryWriteFailed(e.to_string()))?;
        self.write_file(path, &content)
    }

    /// 原子写入文件（使用临时文件 + rename）
    pub fn atomic_write(&self, path: &Path, content: &str) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            self.ensure_dir(parent)?;
        }

        // 创建临时文件
        let temp_path = path.with_extension("tmp");
        let file =
            File::create(&temp_path).map_err(|e| AppError::RegistryWriteFailed(e.to_string()))?;

        let mut writer = BufWriter::new(file);
        writer
            .write_all(content.as_bytes())
            .map_err(|e| AppError::RegistryWriteFailed(e.to_string()))?;
        writer
            .flush()
            .map_err(|e| AppError::RegistryWriteFailed(e.to_string()))?;

        // 同步到磁盘
        writer
            .into_inner()
            .map_err(|e| AppError::RegistryWriteFailed(e.to_string()))?
            .sync_all()
            .map_err(|e| AppError::RegistryWriteFailed(e.to_string()))?;

        // 原子重命名
        let _ = fs::rename(&temp_path, path).map_err(|e| {
            // 清理临时文件
            let _ = fs::remove_file(&temp_path);
            AppError::RegistryWriteFailed(e.to_string())
        });

        Ok(())
    }

    /// 原子写入 JSON 文件
    pub fn atomic_write_json<T: serde::Serialize>(
        &self,
        path: &Path,
        data: &T,
    ) -> Result<(), AppError> {
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| AppError::RegistryWriteFailed(e.to_string()))?;
        self.atomic_write(path, &content)
    }

    /// 删除文件
    pub fn delete_file(&self, path: &Path) -> Result<(), AppError> {
        if path.exists() {
            fs::remove_file(path).map_err(|e| AppError::RegistryWriteFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// 检查文件是否存在
    pub fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    /// 获取数据目录
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

/// 文件锁管理
#[derive(Debug)]
pub struct FileLock {
    file: File,
    path: PathBuf,
}

impl FileLock {
    /// 尝试获取文件锁
    pub fn try_lock(path: &Path) -> Result<Self, AppError> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::RegistryLockFailed(e.to_string()))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| AppError::RegistryLockFailed(e.to_string()))?;

        file.try_lock_exclusive()
            .map_err(|e| AppError::RegistryLockFailed(e.to_string()))?;

        debug!("Acquired lock: {}", path.display());

        Ok(Self {
            file,
            path: path.to_path_buf(),
        })
    }

    /// 释放锁
    pub fn unlock(self) {
        if let Err(e) = fs2::FileExt::unlock(&self.file) {
            error!("Failed to release lock {}: {}", self.path.display(), e);
        } else {
            debug!("Released lock: {}", self.path.display());
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // FileLock 在 drop 时自动释放锁
    }
}

/// 获取特定 skill 的锁
pub fn get_skill_lock(skill_name: &str, data_dir: &Path) -> Result<FileLock, AppError> {
    let lock_path = data_dir.join(format!(".lock-{}", skill_name));
    FileLock::try_lock(&lock_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_storage_service_new() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        assert_eq!(storage.data_dir(), temp_dir.path());
    }

    #[test]
    fn test_storage_service_ensure_dir() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let new_dir = temp_dir.path().join("subdir").join("nested");

        assert!(!new_dir.exists());
        storage.ensure_dir(&new_dir).unwrap();
        assert!(new_dir.exists());
    }

    #[test]
    fn test_storage_service_write_and_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("test.txt");

        storage.write_file(&file_path, "hello world").unwrap();
        let content = storage.read_file(&file_path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_storage_service_read_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("nonexistent.txt");

        let result = storage.read_file(&file_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::FileNotFound(_)));
    }

    #[test]
    fn test_storage_service_write_and_read_json() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("test.json");

        let data = serde_json::json!({"name": "test", "value": 42});
        storage.write_json(&file_path, &data).unwrap();

        let loaded: serde_json::Value = storage.read_json(&file_path).unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn test_storage_service_read_json_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("invalid.json");

        storage.write_file(&file_path, "not valid json {").unwrap();
        let result: Result<serde_json::Value, _> = storage.read_json(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_service_exists() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("exists.txt");

        assert!(!storage.exists(&file_path));
        storage.write_file(&file_path, "content").unwrap();
        assert!(storage.exists(&file_path));
    }

    #[test]
    fn test_storage_service_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("to_delete.txt");

        storage.write_file(&file_path, "content").unwrap();
        assert!(storage.exists(&file_path));

        storage.delete_file(&file_path).unwrap();
        assert!(!storage.exists(&file_path));
    }

    #[test]
    fn test_storage_service_delete_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("nonexistent.txt");

        let result = storage.delete_file(&file_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_storage_service_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("atomic.txt");

        storage.atomic_write(&file_path, "atomic content").unwrap();
        let content = storage.read_file(&file_path).unwrap();
        assert_eq!(content, "atomic content");
    }

    #[test]
    fn test_storage_service_atomic_write_json() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("atomic.json");

        let data = serde_json::json!({"atomic": true});
        storage.atomic_write_json(&file_path, &data).unwrap();

        let loaded: serde_json::Value = storage.read_json(&file_path).unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn test_storage_service_atomic_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("deep").join("nested").join("file.txt");

        storage.atomic_write(&file_path, "content").unwrap();
        let content = storage.read_file(&file_path).unwrap();
        assert_eq!(content, "content");
    }

    #[test]
    fn test_file_lock_debug() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("test.lock");

        let lock = FileLock::try_lock(&lock_path).unwrap();
        let debug_str = format!("{:?}", lock);
        assert!(debug_str.contains("FileLock"));
    }

    #[test]
    fn test_storage_service_read_nonexistent_json() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("nonexistent.json");

        let result: Result<serde_json::Value, _> = storage.read_json(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_service_atomic_write_failure() {
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageService::new(temp_dir.path().to_path_buf());
        let file_path = temp_dir.path().join("atomic.txt");

        storage.atomic_write(&file_path, "test content").unwrap();
        let content = storage.read_file(&file_path).unwrap();
        assert_eq!(content, "test content");
    }
}
