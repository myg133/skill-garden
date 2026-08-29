//! 测试公共模块

use std::path::PathBuf;
use tempfile::TempDir;

#[allow(dead_code)]
pub fn create_test_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

#[allow(dead_code)]
pub fn create_test_data_dir() -> (TempDir, PathBuf) {
    let temp_dir = create_test_temp_dir();
    let data_dir = temp_dir.path().join("data");

    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    (temp_dir, data_dir)
}
