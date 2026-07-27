//! Onboarding handlers
//!
//! The `GET /api/v1/onboarding/setup-skill` endpoint returns the
//! `SKILL.md` content for the `skill-garden-setup` skill along with
//! supporting metadata. The endpoint requires only an authenticated
//! session — it is not bound to any API Key and is available to all
//! logged-in users.

use axum::{http::header, response::IntoResponse, Json};
use serde_json::json;

use crate::api::jwt::AgentContext;
use crate::services::setup_skill::SetupSkillBuilder;

/// `GET /api/v1/onboarding/setup-skill`
///
/// Returns the embedded `SKILL.md` document and associated metadata.
/// Requires a valid session JWT.
pub async fn get_setup_skill_handler(
    _agent_context: AgentContext,
) -> Result<impl IntoResponse, crate::api::error::ApiError> {
    let doc = SetupSkillBuilder::build();

    let body = json!({
        "filename": doc.filename,
        "directory_name": doc.directory_name,
        "content_type": doc.content_type,
        "encoding": doc.encoding,
        "content": doc.content,
        "agent_prompt": doc.agent_prompt,
        "server_url": doc.server_url,
        "mcp_url": doc.mcp_url,
        "sse_url": doc.sse_url,
        "version": doc.version,
    });

    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::PRAGMA, "no-cache"),
        ],
        Json(body),
    ))
}
