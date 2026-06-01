//! Session Service

use uuid::Uuid;
use crate::db::repositories::session::{SessionRepository, NewSession, Session as SessionRepo};
use crate::db::repositories::agent::AgentRepository;
use crate::db::repositories::session_context::{
    SessionContextRepository, NewSessionContext, NewSessionSkill, NewToolExecution,
    SessionContext, SessionSkillState, SessionToolExecution,
};
use crate::models::session::{ToolRouter, RouteTarget};
use crate::models::error::AppError;

#[derive(Clone)]
pub struct SessionService {
    session_repo: SessionRepository,
    agent_repo: AgentRepository,
    context_repo: SessionContextRepository,
}

impl std::fmt::Debug for SessionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionService").finish()
    }
}

impl SessionService {
    pub fn new(session_repo: SessionRepository, agent_repo: AgentRepository, context_repo: SessionContextRepository) -> Self {
        Self { session_repo, agent_repo, context_repo }
    }

    pub async fn create_session(&self, agent_id: String, org_id: Uuid) -> Result<SessionRepo, AppError> {
        let new_session = NewSession { agent_id, org_id };

        self.session_repo.create(new_session)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn end_session(&self, session_id: Uuid) -> Result<(), AppError> {
        self.session_repo.end_session(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<Option<SessionRepo>, AppError> {
        self.session_repo.find_by_id(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_sessions(&self, limit: i64, offset: i64, status: Option<&str>) -> Result<Vec<SessionRepo>, AppError> {
        self.session_repo.list_all(limit, offset, status)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_active_session(&self, agent_id: &str) -> Result<Option<SessionRepo>, AppError> {
        let sessions = self.session_repo.find_active_by_agent(agent_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(sessions.into_iter().next())
    }

    pub async fn get_tool_router(&self, session_id: Uuid) -> Result<Option<crate::models::session::ToolRouter>, AppError> {
        let session = self.session_repo.find_by_id(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        match session {
            Some(s) => {
                let router: crate::models::session::ToolRouter = serde_json::from_value(s.tool_router)
                    .map_err(|e| AppError::InternalError(e.to_string()))?;
                Ok(Some(router))
            }
            None => Ok(None),
        }
    }

    pub async fn declare_capabilities(
        &self,
        session_id: Uuid,
        capabilities: Vec<String>,
    ) -> Result<ToolRouter, AppError> {
        let session = self.session_repo.find_by_id(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .ok_or_else(|| AppError::ValidationError(format!("Session {} not found", session_id)))?;

        // Get agent's capabilities from agent record
        let agent = self.agent_repo.find_by_id(&session.agent_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let agent_capabilities = match agent {
            Some(a) => a.capabilities,
            None => Vec::new(),
        };

        // Build tool router based on declared capabilities
        let mut router = ToolRouter::new();

        // Platform tools always route to platform
        let platform_tools = vec!["browse", "qa", "exec", "storage"];
        for tool in &platform_tools {
            router.add_route(tool.to_string(), RouteTarget::Platform);
        }

        // Agent capabilities route to local
        for cap in &agent_capabilities {
            if !platform_tools.contains(&cap.as_str()) {
                router.add_route(cap.clone(), RouteTarget::Local);
            }
        }

        // Declared additional capabilities route to local
        for cap in &capabilities {
            if !platform_tools.contains(&cap.as_str()) {
                if !agent_capabilities.contains(cap) {
                    router.add_route(cap.clone(), RouteTarget::Local);
                }
            }
        }

        // Update session with tool router
        let router_json = serde_json::to_value(&router).map_err(|e| AppError::ValidationError(e.to_string()))?;
        self.session_repo.update_tool_router(session_id, router_json)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(router)
    }

    // Session Context Methods

    pub async fn set_context(&self, session_id: Uuid, key: String, value: serde_json::Value) -> Result<SessionContext, AppError> {
        let new_ctx = NewSessionContext {
            session_id,
            context_key: key,
            context_value: value,
        };
        self.context_repo.create_context(new_ctx)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_context(&self, session_id: Uuid, key: &str) -> Result<Option<SessionContext>, AppError> {
        self.context_repo.get_context(session_id, key)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_contexts(&self, session_id: Uuid) -> Result<Vec<SessionContext>, AppError> {
        self.context_repo.list_contexts(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn delete_context(&self, session_id: Uuid, key: &str) -> Result<(), AppError> {
        self.context_repo.delete_context(session_id, key)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    // Session Skill State Methods

    pub async fn load_skill(&self, session_id: Uuid, skill_id: String, skill_state: serde_json::Value) -> Result<SessionSkillState, AppError> {
        let new_skill = NewSessionSkill {
            session_id,
            skill_id: skill_id.clone(),
            skill_state,
            status: "loaded".to_string(),
        };
        let result = self.context_repo.load_skill(new_skill)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        tracing::info!("Skill {} loaded in session {}", skill_id, session_id);
        Ok(result)
    }

    pub async fn unload_skill(&self, session_id: Uuid, skill_id: &str) -> Result<(), AppError> {
        self.context_repo.unload_skill(session_id, skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        tracing::info!("Skill {} unloaded from session {}", skill_id, session_id);
        Ok(())
    }

    pub async fn get_session_skill(&self, session_id: Uuid, skill_id: &str) -> Result<Option<SessionSkillState>, AppError> {
        self.context_repo.get_session_skill(session_id, skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_session_skills(&self, session_id: Uuid) -> Result<Vec<SessionSkillState>, AppError> {
        self.context_repo.list_session_skills(session_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn update_skill_state(&self, session_id: Uuid, skill_id: &str, skill_state: serde_json::Value) -> Result<(), AppError> {
        self.context_repo.update_skill_state(session_id, skill_id, skill_state)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    // Tool Execution History Methods

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
        self.context_repo.record_tool_execution(execution)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_tool_execution_history(&self, session_id: Uuid, limit: i64) -> Result<Vec<SessionToolExecution>, AppError> {
        self.context_repo.get_tool_execution_history(session_id, limit)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
