//! Organization Tool Service

use crate::db::repositories::org_tool::{NewOrgTool, OrgTool as OrgToolRepo, OrgToolRepository};
use crate::models::error::AppError;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Clone)]
pub struct OrgToolService {
    org_tool_repo: OrgToolRepository,
}

impl std::fmt::Debug for OrgToolService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrgToolService").finish()
    }
}

impl OrgToolService {
    pub fn new(org_tool_repo: OrgToolRepository) -> Self {
        Self { org_tool_repo }
    }

    pub async fn register_tool(
        &self,
        org_id: Uuid,
        tool_id: String,
        name: String,
        description: String,
        schema: JsonValue,
        implementation: JsonValue,
    ) -> Result<OrgToolRepo, AppError> {
        let new_tool = NewOrgTool {
            tool_id: tool_id.clone(),
            org_id,
            name,
            description,
            schema,
            implementation,
        };

        self.org_tool_repo
            .create(new_tool)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn approve_tool(&self, tool_id: Uuid) -> Result<(), AppError> {
        self.org_tool_repo
            .update_status(tool_id, "approved")
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn reject_tool(&self, tool_id: Uuid) -> Result<(), AppError> {
        self.org_tool_repo
            .update_status(tool_id, "rejected")
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_org_tools(&self, org_id: Uuid) -> Result<Vec<OrgToolRepo>, AppError> {
        self.org_tool_repo
            .find_by_org(org_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_all(&self) -> Result<Vec<OrgToolRepo>, AppError> {
        self.org_tool_repo
            .find_all()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn list_approved_tools(&self, org_id: Uuid) -> Result<Vec<OrgToolRepo>, AppError> {
        self.org_tool_repo
            .find_approved_by_org(org_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn get_tool(&self, tool_id: Uuid) -> Result<Option<OrgToolRepo>, AppError> {
        self.org_tool_repo
            .find_by_id(tool_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    pub async fn delete(&self, tool_id: Uuid) -> Result<(), AppError> {
        self.org_tool_repo
            .delete(tool_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
