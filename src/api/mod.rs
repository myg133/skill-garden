//! REST API 模块
//!
//! 提供 HTTP REST 接口

pub mod error;
pub mod handlers;
pub mod http_state;
pub mod jwt;
pub mod auth;
pub mod models;
pub mod routes;

pub use error::{ApiError, ApiResult};
pub use handlers::*;
pub use http_state::{AppRouterState, HttpState, SseState};
pub use jwt::{AgentContext, JwtAuth, generate_token};
pub use routes::create_api_router;