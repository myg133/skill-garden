//! Session Service — manages MCP connection sessions per identity

use crate::db::repositories::session::{NewSession, Session as SessionRepo, SessionRepository};
use crate::db::repositories::session_context::{
    NewSessionContext, NewSessionSkill, NewToolExecution, SessionContext, SessionContextRepository,
    SessionSkillState, SessionToolExecution,
};
use crate::models::error::AppError;
use crate::models::session::{RouteTarget, ToolRouter};
use uuid::Uuid;

#[derive(Clone)]
pub struct SessionService {
    session_repo: SessionRepository,
    context_repo: SessionContextRepository,
}

impl std::fmt::Debug for SessionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionService").finish()
    }
}

impl SessionService {
    pub fn new(session_repo: SessionRepository, context_repo: SessionContextRepository) -> Self {
        Self {
            session_repo,
            context_repo,
        }
    }

    // ─── Session lifecycle ───────────────────────────────────

    /// Create a new session for the given identity + organization.
    pub async fn create_session(
        &self,
        identity_id: Uuid,
        org_id: Uuid,
    ) -> Result<SessionRepo, AppError> {
        let new_session = NewSession {
            identity_id,
            org_id,
        };
        self.session_repo
            .create(new_session)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Find an existing active session for the identity, or create a new one.
    /// Returns (session, is_new).
    pub async fn find_or_create_session(
        &self,
        identity_id: Uuid,
        org_id: Uuid,
    ) -> Result<SessionRepo, AppError> {
        let existing = self.get_active_session(identity_id).await?;
        if let Some(session) = existing {
            tracing::debug!(
                "Reusing existing session {} for identity {}",
                session.id,
                identity_id
            );
            return Ok(session);
        }
        let session = self.create_session(identity_id, org_id).await?;
        tracing::info!(
            "Created new session {} for identity {}",
            session.id,
            identity_id
        );
        Ok(session)
    }

    pub async fn end_session(&self, session_id: Uuid) -> Result<(), AppError> {
        self.session_repo
            .end_session(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<Option<SessionRepo>, AppError> {
        self.session_repo
            .find_by_id(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_sessions(
        &self,
        limit: i64,
        offset: i64,
        status: Option<&str>,
    ) -> Result<Vec<SessionRepo>, AppError> {
        self.session_repo
            .list_all(limit, offset, status)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_active_session(
        &self,
        identity_id: Uuid,
    ) -> Result<Option<SessionRepo>, AppError> {
        let sessions = self
            .session_repo
            .find_active_by_identity(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(sessions.into_iter().next())
    }

    /// Update last_active_at to now (call on each MCP request).
    pub async fn touch_session(&self, session_id: Uuid) -> Result<(), AppError> {
        self.session_repo
            .touch(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// End all sessions idle longer than `idle_secs` seconds.
    pub async fn end_idle_sessions(&self, idle_secs: i64) -> Result<usize, AppError> {
        self.session_repo
            .end_idle_sessions(idle_secs)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    // ─── Tool router ────────────────────────────────────────

    pub async fn get_tool_router(
        &self,
        session_id: Uuid,
    ) -> Result<Option<crate::models::session::ToolRouter>, AppError> {
        let session = self
            .session_repo
            .find_by_id(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        match session {
            Some(s) => {
                let router: crate::models::session::ToolRouter =
                    serde_json::from_value(s.tool_router)
                        .map_err(|e| AppError::InternalError(e.to_string()))?;
                Ok(Some(router))
            }
            None => Ok(None),
        }
    }

    /// Declare capabilities for a session — builds the tool router.
    /// (No longer looks up agent records; only routes based on declared capabilities.)
    pub async fn declare_capabilities(
        &self,
        session_id: Uuid,
        capabilities: Vec<String>,
    ) -> Result<ToolRouter, AppError> {
        let session = self
            .session_repo
            .find_by_id(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .ok_or_else(|| {
                AppError::ValidationError(format!("Session {} not found", session_id))
            })?;

        let _ = session; // session is validated to exist

        let mut router = ToolRouter::new();

        // Platform tools always route to platform
        let platform_tools = vec!["browse", "qa", "exec", "storage"];
        for tool in &platform_tools {
            router.add_route(tool.to_string(), RouteTarget::Platform);
        }

        // Declared capabilities route to local
        for cap in &capabilities {
            if !platform_tools.contains(&cap.as_str()) {
                router.add_route(cap.clone(), RouteTarget::Local);
            }
        }

        let router_json =
            serde_json::to_value(&router).map_err(|e| AppError::ValidationError(e.to_string()))?;
        self.session_repo
            .update_tool_router(session_id, router_json)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(router)
    }

    // ─── Session Context ────────────────────────────────────

    pub async fn set_context(
        &self,
        session_id: Uuid,
        key: String,
        value: serde_json::Value,
    ) -> Result<SessionContext, AppError> {
        let new_ctx = NewSessionContext {
            session_id,
            context_key: key,
            context_value: value,
        };
        self.context_repo
            .create_context(new_ctx)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_context(
        &self,
        session_id: Uuid,
        key: &str,
    ) -> Result<Option<SessionContext>, AppError> {
        self.context_repo
            .get_context(session_id, key)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_contexts(&self, session_id: Uuid) -> Result<Vec<SessionContext>, AppError> {
        self.context_repo
            .list_contexts(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn delete_context(&self, session_id: Uuid, key: &str) -> Result<(), AppError> {
        self.context_repo
            .delete_context(session_id, key)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    // ─── Session Skill State ────────────────────────────────

    pub async fn load_skill(
        &self,
        session_id: Uuid,
        skill_id: String,
        skill_state: serde_json::Value,
    ) -> Result<SessionSkillState, AppError> {
        let new_skill = NewSessionSkill {
            session_id,
            skill_id: skill_id.clone(),
            skill_state,
            status: "loaded".to_string(),
        };
        let result = self
            .context_repo
            .load_skill(new_skill)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        tracing::info!("Skill {} loaded in session {}", skill_id, session_id);
        Ok(result)
    }

    pub async fn unload_skill(&self, session_id: Uuid, skill_id: &str) -> Result<(), AppError> {
        self.context_repo
            .unload_skill(session_id, skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        tracing::info!("Skill {} unloaded from session {}", skill_id, session_id);
        Ok(())
    }

    pub async fn get_session_skill(
        &self,
        session_id: Uuid,
        skill_id: &str,
    ) -> Result<Option<SessionSkillState>, AppError> {
        self.context_repo
            .get_session_skill(session_id, skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_session_skills(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<SessionSkillState>, AppError> {
        self.context_repo
            .list_session_skills(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update_skill_state(
        &self,
        session_id: Uuid,
        skill_id: &str,
        skill_state: serde_json::Value,
    ) -> Result<(), AppError> {
        self.context_repo
            .update_skill_state(session_id, skill_id, skill_state)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    // ─── Tool Execution History ─────────────────────────────

    pub async fn record_tool_execution(
        &self,
        session_id: Uuid,
        tool_id: String,
        tool_type: String,
        parameters: serde_json::Value,
        result: Option<serde_json::Value>,
        success: bool,
        execution_time_ms: Option<i32>,
        error_message: Option<String>,
    ) -> Result<SessionToolExecution, AppError> {
        let execution = NewToolExecution {
            session_id,
            tool_id,
            tool_type,
            parameters,
            result,
            success,
            execution_time_ms,
            error_message,
        };
        self.context_repo
            .record_tool_execution(execution)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_tool_execution_history(
        &self,
        session_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SessionToolExecution>, AppError> {
        self.context_repo
            .get_tool_execution_history(session_id, limit)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
