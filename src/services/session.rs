//! Session Service

use uuid::Uuid;
use crate::db::repositories::session::{SessionRepository, NewSession, Session as SessionRepo};
use crate::db::repositories::agent::AgentRepository;
use crate::models::session::{ToolRouter, RouteTarget};
use crate::models::error::AppError;

#[derive(Clone)]
pub struct SessionService {
    session_repo: SessionRepository,
    agent_repo: AgentRepository,
}

impl std::fmt::Debug for SessionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionService").finish()
    }
}

impl SessionService {
    pub fn new(session_repo: SessionRepository, agent_repo: AgentRepository) -> Self {
        Self { session_repo, agent_repo }
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
}
