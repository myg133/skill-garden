//! Onboarding handlers
//!
//! The `GET /api/v1/onboarding/setup-skill` endpoint returns the
//! `SKILL.md` content for the `skill-garden-setup` skill along with
//! supporting metadata. The endpoint is intentionally public so that
//! prospective users can preview the installation guide before they
//! have an API key.

use axum::{http::header, response::IntoResponse, Json};
use serde_json::json;

use crate::services::setup_skill::SetupSkillBuilder;

/// `GET /api/v1/onboarding/setup-skill`
///
/// Returns the embedded `SKILL.md` document and associated metadata.
/// Public endpoint — no authentication required.
pub async fn get_setup_skill_handler() -> impl IntoResponse {
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

    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::PRAGMA, "no-cache"),
        ],
        Json(body),
    )
}
