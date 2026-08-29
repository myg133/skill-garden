//! Skill Garden CLI — 终端交互工具
//!
//! 通过 HTTP REST API 与 Skill Garden 服务端通信。
//! 不依赖 PostgreSQL、Tantivy、Docker 等服务端组件。

pub mod client;
pub mod commands;
pub mod config;
